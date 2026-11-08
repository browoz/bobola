mod api;
#[cfg(feature = "benchmark-harness")]
mod benchmark;
mod host;
#[cfg(windows)]
mod native_popup;
#[cfg(windows)]
mod native_surface;
mod resources;
#[cfg(debug_assertions)]
mod smoke;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    process::Stdio,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};
use workspace_core::{
    Result,
    interface::InterfacePreferences,
    lifecycle::Runtime,
    model::{NewService, Preferences, Service},
    popup::{PopupActivity, PopupState, summarize, with_popups},
    protection::Protection,
    store::{Backup, Snapshot, Store},
    surface::SurfaceLayout,
};

#[derive(Clone)]
pub struct AppState {
    pub data_dir: Arc<std::path::PathBuf>,
    pub store: Arc<Mutex<Store>>,
    pub protection: Arc<Mutex<Protection>>,
    pub runtime: Arc<Mutex<BTreeMap<String, Runtime>>>,
    pub services: Arc<Mutex<BTreeMap<String, Service>>>,
    /// Activity of open popups by account id, then popup id. Separate from `runtime`
    /// so popup page state never overwrites the main view's dirty/media flags.
    pub popups: Arc<Mutex<BTreeMap<String, BTreeMap<u64, PopupState>>>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MusicStatus {
    available: bool,
    running: bool,
    title: Option<String>,
    artist: Option<String>,
    paused: Option<bool>,
    volume: Option<i64>,
    error: Option<String>,
}

fn music_binary(app: &tauri::AppHandle) -> std::path::PathBuf {
    let resource = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("ytt.exe"));
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("ytt.exe")));
    resource
        .filter(|path| path.is_file())
        .or_else(|| adjacent.filter(|path| path.is_file()))
        .unwrap_or_else(|| std::path::PathBuf::from("ytt.exe"))
}

async fn run_music(app: &tauri::AppHandle, arguments: &[String]) -> Result<String> {
    let binary = music_binary(app);
    let mut process = tokio::process::Command::new(&binary);
    process
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    process.creation_flags(0x08000000);
    let output = process
        .output()
        .await
        .map_err(|error| format!("YuTuTui is unavailable: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "YuTuTui could not complete that action.".into()
        } else {
            message
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_music_status(text: &str) -> MusicStatus {
    let response: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
    let status = response.get("status").unwrap_or(&response);
    MusicStatus {
        available: true,
        running: response
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        title: status
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        artist: status
            .get("artist")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        paused: status.get("paused").and_then(serde_json::Value::as_bool),
        volume: status.get("volume").and_then(serde_json::Value::as_i64),
        error: response
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

#[tauri::command]
async fn music_status(window: tauri::WebviewWindow) -> Result<MusicStatus> {
    shell(&window)?;
    let binary = music_binary(window.app_handle());
    if !binary.is_file() && binary == std::path::Path::new("ytt.exe") {
        return Ok(MusicStatus {
            available: false,
            running: false,
            title: None,
            artist: None,
            paused: None,
            volume: None,
            error: None,
        });
    }
    match run_music(
        window.app_handle(),
        &["-r".into(), "status".into(), "--json".into()],
    )
    .await
    {
        Ok(output) => Ok(parse_music_status(&output)),
        Err(error) => Ok(MusicStatus {
            available: true,
            running: false,
            title: None,
            artist: None,
            paused: None,
            volume: None,
            error: Some(error),
        }),
    }
}

#[tauri::command]
async fn music_command(
    window: tauri::WebviewWindow,
    action: String,
    query: Option<String>,
) -> Result<MusicStatus> {
    shell(&window)?;
    let mut arguments = vec!["-r".to_string()];
    match action.as_str() {
        "play" => {
            let query = query.unwrap_or_default();
            if query.trim().is_empty() {
                arguments.push("pp".into());
            } else {
                arguments.extend(["play".into(), query]);
            }
        }
        "pause" => arguments.push("pp".into()),
        "next" => arguments.push("next".into()),
        "previous" => arguments.push("prev".into()),
        "volumeUp" => arguments.push("up".into()),
        "volumeDown" => arguments.push("down".into()),
        _ => return Err("Unknown music action.".into()),
    }
    arguments.push("--json".into());
    if action == "play" {
        let _ = run_music(window.app_handle(), &["daemon".into(), "start".into()]).await;
    }
    let output = run_music(window.app_handle(), &arguments).await?;
    Ok(parse_music_status(&output))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    #[serde(flatten)]
    snapshot: Snapshot,
    interface_preferences: InterfacePreferences,
    runtime: BTreeMap<String, Runtime>,
    filter_revision: String,
    native: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewLayout {
    pub service_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "An internal operation failed. Restart the app before continuing.".into()
}
fn shell(window: &tauri::WebviewWindow) -> Result<()> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("This command is restricted to the local shell.".into())
    }
}

#[tauri::command]
async fn resource_sample(window: tauri::WebviewWindow) -> Result<resources::ResourceSample> {
    shell(&window)?;
    #[cfg(windows)]
    {
        let (send, receive) = tokio::sync::oneshot::channel();
        window
            .with_webview(move |webview| {
                let _ = send.send(resources::environment_pids(&webview.environment()));
            })
            .map_err(|e| e.to_string())?;
        let mut roots = receive.await.map_err(|e| e.to_string())??;
        roots.extend(host::browser_process_ids(window.app_handle()).await?);
        roots.push(std::process::id());
        tauri::async_runtime::spawn_blocking(move || resources::sample_roots(&roots))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    resources::sample()
}
fn sync_services(state: &AppState) -> Result<()> {
    let snapshot = state.store.lock().map_err(lock_error)?.snapshot()?;
    *state.services.lock().map_err(lock_error)? = snapshot
        .services
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect();
    Ok(())
}

#[tauri::command]
fn bootstrap(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<Bootstrap> {
    shell(&window)?;
    // One lock for both reads so the snapshot and sidecar come from the same moment.
    // A damaged sidecar row makes bootstrap fail visibly instead of being reset.
    let (snapshot, interface_preferences) = {
        let store = state.store.lock().map_err(lock_error)?;
        (store.snapshot()?, store.interface_preferences()?)
    };
    Ok(Bootstrap {
        snapshot,
        interface_preferences,
        runtime: state.runtime.lock().map_err(lock_error)?.clone(),
        filter_revision: state
            .protection
            .lock()
            .map_err(lock_error)?
            .revision
            .clone(),
        native: cfg!(windows),
    })
}
#[tauri::command]
fn add_service(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    input: NewService,
) -> Result<Service> {
    shell(&window)?;
    let service = state.store.lock().map_err(lock_error)?.add_service(input)?;
    sync_services(&state)?;
    api::snapshot_changed(window.app_handle());
    Ok(service)
}
#[tauri::command]
async fn update_service(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    service: Service,
) -> Result<()> {
    shell(&window)?;
    if !state
        .services
        .lock()
        .map_err(lock_error)?
        .contains_key(&service.id)
    {
        return Err("Service not found.".into());
    }
    state
        .store
        .lock()
        .map_err(lock_error)?
        .save_service(&service)?;
    sync_services(&state)?;
    api::snapshot_changed(window.app_handle());
    #[cfg(windows)]
    if !service.protection {
        host::clear_cosmetics(window.app_handle(), service.id.clone()).await?;
    }
    Ok(())
}
#[tauri::command]
async fn delete_service(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    shell(&window)?;
    host::close(
        &window.app_handle().clone(),
        id.clone(),
        state.inner().clone(),
        host::CloseReason::Remove,
    )
    .await?;
    state
        .store
        .lock()
        .map_err(lock_error)?
        .delete_service(&id)?;
    sync_services(&state)?;
    api::snapshot_changed(window.app_handle());
    Ok(())
}
#[tauri::command]
fn add_workspace(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    name: String,
) -> Result<workspace_core::model::Workspace> {
    shell(&window)?;
    let workspace = state
        .store
        .lock()
        .map_err(lock_error)?
        .add_workspace(&name)?;
    api::snapshot_changed(window.app_handle());
    Ok(workspace)
}
#[tauri::command]
fn save_preferences(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    preferences: Preferences,
) -> Result<()> {
    shell(&window)?;
    state
        .store
        .lock()
        .map_err(lock_error)?
        .set_preferences(&preferences)?;
    // Without a sidecar row the interface theme follows this value, so the UI re-reads
    // everything through bootstrap.
    api::snapshot_changed(window.app_handle());
    Ok(())
}
/// Saves the interface sidecar. `closeBehavior: "tray"` is refused until the next milestone.
#[tauri::command]
fn save_interface_preferences(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    preferences: InterfacePreferences,
) -> Result<()> {
    shell(&window)?;
    state
        .store
        .lock()
        .map_err(lock_error)?
        .set_interface_preferences(&preferences)?;
    api::interface_preferences_changed(window.app_handle(), preferences);
    Ok(())
}
#[tauri::command]
async fn set_layout(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    layout: Vec<ViewLayout>,
) -> Result<()> {
    shell(&window)?;
    #[cfg(debug_assertions)]
    if std::env::var_os("SERVICE_WORKSPACE_TEST_OVERLAY").is_some() {
        return Ok(());
    }
    if layout.len() > 5
        || layout.iter().any(|item| {
            ![item.x, item.y, item.width, item.height]
                .iter()
                .all(|v| v.is_finite())
                || item.x < 0.0
                || item.y < 0.0
                || item.width < 1.0
                || item.height < 1.0
                || item.width > 20000.0
                || item.height > 20000.0
        })
    {
        return Err("Invalid service layout.".into());
    }
    host::show(&window.app_handle().clone(), layout, state.inner().clone()).await
}
/// Atomic native surface layout. `set_layout` remains for compatibility.
#[tauri::command]
async fn set_surface_layout(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    layout: SurfaceLayout,
) -> Result<()> {
    shell(&window)?;
    #[cfg(debug_assertions)]
    if std::env::var_os("SERVICE_WORKSPACE_TEST_OVERLAY").is_some() {
        return Ok(());
    }
    host::show_surface(&window.app_handle().clone(), layout, state.inner().clone()).await
}
#[tauri::command]
async fn workspace_command(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    command: api::WorkspaceCommand,
) -> Result<()> {
    shell(&window)?;
    api::dispatch(&window.app_handle().clone(), state.inner().clone(), command).await
}
#[tauri::command]
async fn unload_service(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    shell(&window)?;
    host::close(
        &window.app_handle().clone(),
        id,
        state.inner().clone(),
        host::CloseReason::User,
    )
    .await
}
#[tauri::command]
async fn reload_service(window: tauri::WebviewWindow, id: String) -> Result<()> {
    shell(&window)?;
    host::reload(&window.app_handle().clone(), id).await
}
#[tauri::command]
fn export_backup(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<String> {
    shell(&window)?;
    // Format 2: {version:2, snapshot, interfacePreferences}. No cookies, sessions or keys.
    let backup = state.store.lock().map_err(lock_error)?.backup()?;
    serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())
}
#[tauri::command]
async fn import_backup(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    text: String,
) -> Result<()> {
    shell(&window)?;
    // Accepts format 2 and legacy format 1. Size, version and unknown fields are checked here.
    let backup = Backup::parse(&text)?;
    if state
        .runtime
        .lock()
        .map_err(lock_error)?
        .values()
        .any(|s| s.loaded)
    {
        return Err("Close loaded services before importing a backup.".into());
    }
    // One transaction covers services, workspaces, legacy preferences and the sidecar.
    let interface_preferences = {
        let mut store = state.store.lock().map_err(lock_error)?;
        store.restore_backup(backup)?;
        store.interface_preferences()
    };
    sync_services(&state)?;
    let app = window.app_handle();
    api::snapshot_changed(app);
    // The restore already committed, so a failed re-read must not turn it into an error.
    if let Ok(preferences) = interface_preferences {
        api::interface_preferences_changed(app, preferences);
    }
    Ok(())
}
#[tauri::command]
async fn refresh_filters(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<String> {
    shell(&window)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;
    let mut combined = String::new();
    for url in [
        "https://easylist.to/easylist/easylist.txt",
        "https://easylist.to/easylist/easyprivacy.txt",
    ] {
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Could not download filters: {e}"))?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if body.len() + chunk.len() > 6 * 1024 * 1024 {
                return Err("Filter download exceeds its size limit.".into());
            }
            body.extend_from_slice(&chunk);
        }
        let text =
            String::from_utf8(body).map_err(|_| "Filter response is not UTF-8.".to_string())?;
        if !text.trim_start().starts_with("[Adblock") {
            return Err(
                "Filter server returned an unexpected response; previous filters remain active."
                    .into(),
            );
        }
        combined.push_str(&text);
        combined.push('\n');
    }
    let owned = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let next = owned
            .store
            .lock()
            .map_err(lock_error)?
            .apply_filters(&combined)?;
        let revision = next.revision.clone();
        *owned.protection.lock().map_err(lock_error)? = next;
        Ok(revision)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn rollback_filters(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<String> {
    shell(&window)?;
    let previous = state.store.lock().map_err(lock_error)?.rollback_filters()?;
    let revision = previous.revision.clone();
    *state.protection.lock().map_err(lock_error)? = previous;
    Ok(revision)
}

/// Environment variables that make WebView2 ignore or replace the profile directory that a test
/// or benchmark run asks for. An isolated run refuses to start while any of them is set.
const WEBVIEW2_OVERRIDES: [&str; 4] = [
    "WEBVIEW2_USER_DATA_FOLDER",
    "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
    "WEBVIEW2_RELEASE_CHANNEL_PREFERENCE",
    "WEBVIEW2_CHANNEL_SEARCH_KIND",
];

/// Owned root directory of a test or benchmark run, or `None` for a normal run.
/// A debug test directory wins over a benchmark directory, as it does for the store location.
fn isolated_root() -> Option<std::path::PathBuf> {
    #[cfg(debug_assertions)]
    let test = std::env::var_os("SERVICE_WORKSPACE_TEST_DIR");
    #[cfg(not(debug_assertions))]
    let test: Option<std::ffi::OsString> = None;
    #[cfg(feature = "benchmark-harness")]
    let bench = {
        let folder = std::path::PathBuf::from(
            std::env::var_os("SERVICE_WORKSPACE_BENCHMARK_DIR")
                .expect("An isolated benchmark directory is required"),
        );
        assert!(folder.is_absolute(), "Benchmark directory must be absolute");
        Some(folder.into_os_string())
    };
    #[cfg(not(feature = "benchmark-harness"))]
    let bench: Option<std::ffi::OsString> = None;
    let root = std::path::absolute(test.or(bench)?)
        .expect("The test or benchmark directory must be a usable path");
    for name in WEBVIEW2_OVERRIDES {
        assert!(
            std::env::var_os(name).is_none(),
            "{name} is set and would redirect the isolated WebView2 profile. Unset it for test runs."
        );
    }
    Some(root)
}

/// Creates the shell window by hand so its WebView2 profile is an explicit absolute path.
/// `WindowConfig.data_directory` cannot do this: Tauri resolves it as a relative path under its
/// own local data directory and does not honor an absolute one.
fn create_shell_window(
    app: &tauri::AppHandle,
    mut config: tauri::utils::config::WindowConfig,
    profile: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(profile)?;
    config.data_directory = None;
    tauri::WebviewWindowBuilder::from_config(app, &config)?
        .data_directory(profile.to_path_buf())
        .build()?;
    Ok(())
}

pub fn run() {
    let mut context = tauri::generate_context!();
    // Test and benchmark runs keep the shell WebView2 profile in `<root>/shell-browser`, so the
    // default WebView2 directory of a running user instance is never opened. For those runs only,
    // the shell window is removed from the generated config (so Tauri does not create it) and is
    // created in `setup` instead. Normal runs leave the config untouched.
    let shell_profile = isolated_root().map(|root| root.join("shell-browser"));
    #[cfg_attr(not(feature = "benchmark-harness"), allow(unused_mut))]
    let mut shell_window = shell_profile.as_ref().map(|_| {
        let windows = &mut context.config_mut().app.windows;
        assert!(!windows.is_empty(), "The shell window is not configured");
        windows.remove(0)
    });
    #[cfg(feature = "benchmark-harness")]
    {
        if let Some(window) = shell_window.as_mut() {
            window.visible = true;
            window.width = 1320.0;
            window.height = 860.0;
            if let Ok(origin) = std::env::var("SERVICE_WORKSPACE_BENCHMARK_ORIGIN") {
                let url = url::Url::parse(&origin)
                    .and_then(|base| base.join("/shell"))
                    .expect("Invalid benchmark fixture origin");
                window.url = tauri::utils::config::WebviewUrl::External(url);
            }
        }
    }
    tauri::Builder::default()
        .setup(move |app| {
            #[cfg(not(feature = "benchmark-harness"))]
            let folder = app.path().app_local_data_dir()?;
            #[cfg(feature = "benchmark-harness")]
            let folder = std::path::PathBuf::from(
                std::env::var_os("SERVICE_WORKSPACE_BENCHMARK_DIR")
                    .expect("Missing benchmark directory"),
            );
            #[cfg(all(debug_assertions, not(feature = "benchmark-harness")))]
            let folder = std::env::var_os("SERVICE_WORKSPACE_TEST_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(folder);
            std::fs::create_dir_all(&folder)?;
            let store =
                Store::open(&folder.join("workspace.sqlite")).map_err(std::io::Error::other)?;
            let engine = Protection::compile(&store.filter_text().map_err(std::io::Error::other)?)
                .map_err(std::io::Error::other)?;
            let services = store
                .snapshot()
                .map_err(std::io::Error::other)?
                .services
                .into_iter()
                .map(|s| (s.id.clone(), s))
                .collect();
            let state = AppState {
                data_dir: Arc::new(folder),
                store: Arc::new(Mutex::new(store)),
                protection: Arc::new(Mutex::new(engine)),
                runtime: Arc::new(Mutex::new(BTreeMap::new())),
                services: Arc::new(Mutex::new(services)),
                popups: Arc::new(Mutex::new(BTreeMap::new())),
            };
            app.manage(state.clone());
            // Isolated runs only: the shell window must exist before the smoke or benchmark
            // installer looks it up, and after state is managed so early commands can run.
            if let (Some(profile), Some(window)) = (shell_profile, shell_window) {
                create_shell_window(app.handle(), window, &profile)?;
            }
            #[cfg(feature = "benchmark-harness")]
            benchmark::install(app.handle(), state.clone()).map_err(std::io::Error::other)?;
            #[cfg(debug_assertions)]
            smoke::install(app.handle(), state.clone()).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    let Ok(store) = state.store.lock() else { break };
                    let Ok(snapshot) = store.snapshot() else {
                        continue;
                    };
                    drop(store);
                    let activity: BTreeMap<String, PopupActivity> = match state.popups.lock() {
                        Ok(popups) => popups
                            .iter()
                            .map(|(id, open)| (id.clone(), summarize(open.values())))
                            .collect(),
                        Err(_) => break,
                    };
                    let Ok(runtime) = state.runtime.lock() else {
                        break;
                    };
                    // Popup activity is folded in on a copy so main-view flags stay untouched.
                    let merged: BTreeMap<String, Runtime> = runtime
                        .iter()
                        .map(|(id, entry)| {
                            let popup = activity.get(id).copied().unwrap_or_default();
                            (id.clone(), with_popups(entry, popup))
                        })
                        .collect();
                    drop(runtime);
                    let candidates = workspace_core::lifecycle::eviction_candidates_with_timeouts(
                        snapshot.services.iter().filter_map(|service| {
                            merged.get(&service.id).map(|r| {
                                (
                                    service.id.as_str(),
                                    r,
                                    service.keep_live,
                                    service
                                        .idle_minutes
                                        .unwrap_or(snapshot.preferences.idle_minutes)
                                        * 60,
                                )
                            })
                        }),
                        snapshot.preferences.live_budget,
                        now(),
                    );
                    for id in candidates {
                        let handle = handle.clone();
                        let state = state.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = host::close(&handle, id, state, host::CloseReason::Idle).await;
                        });
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            resource_sample,
            add_service,
            update_service,
            delete_service,
            add_workspace,
            save_preferences,
            save_interface_preferences,
            music_status,
            music_command,
            set_layout,
            set_surface_layout,
            workspace_command,
            unload_service,
            reload_service,
            export_backup,
            import_backup,
            refresh_filters,
            rollback_filters
        ])
        .run(context)
        .expect("Bobola could not start");
}
