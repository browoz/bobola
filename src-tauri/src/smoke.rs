//! Debug-only, owned loopback fixtures. Compiled out of release builds.
use crate::{AppState, ViewLayout, host};
use tauri::Manager;
use workspace_core::{
    Result,
    model::NewService,
    protection::{Protection, STARTER_FILTERS},
};

pub fn install(app: &tauri::AppHandle, state: AppState) -> Result<()> {
    if std::env::var_os("SERVICE_WORKSPACE_TEST_DIR").is_none() {
        return Ok(());
    }
    let origin = std::env::var("SERVICE_WORKSPACE_TEST_ORIGIN")
        .map_err(|_| "Missing fixture origin.".to_string())?;
    let parsed = url::Url::parse(&origin).map_err(|e| e.to_string())?;
    if parsed.scheme() != "http" || parsed.host_str() != Some("127.0.0.1") {
        return Err("Native smoke fixtures must use loopback HTTP.".into());
    }
    // The overlay phase hit-tests real screen points, so the shell window must stay visible.
    let overlay = std::env::var_os("SERVICE_WORKSPACE_TEST_OVERLAY").is_some();
    if !overlay {
        app.get_webview_window("main")
            .ok_or("Missing shell window.")?
            .hide()
            .map_err(|e| e.to_string())?;
    }
    let phase = std::env::var("SERVICE_WORKSPACE_TEST_PHASE").unwrap_or_else(|_| "first".into());
    let mut slots = Vec::new();
    {
        let mut store = state.store.lock().map_err(|e| e.to_string())?;
        for (index, role) in ["A", "B"].into_iter().enumerate() {
            let name = format!("Fixture {role}");
            let mut service = if let Some(service) = store
                .snapshot()?
                .services
                .into_iter()
                .find(|s| s.name == name)
            {
                service
            } else {
                store.add_service(NewService {
                    name,
                    url: format!("{origin}/fixture?role={role}&phase={phase}"),
                    workspace_id: "personal".into(),
                    keep_live: Some(true),
                    catalog_id: None,
                })?
            };
            service.url = format!("{origin}/fixture?role={role}&phase={phase}");
            service.protection = !(phase == "restart" && role == "A");
            store.save_service(&service)?;
            slots.push(ViewLayout {
                service_id: service.id,
                x: (index * 510) as f64,
                y: 100.0,
                width: 500.0,
                height: 600.0,
            });
        }
    }
    crate::sync_services(&state)?;
    *state.protection.lock().map_err(|e| e.to_string())? = Protection::compile(&format!(
        "{STARTER_FILTERS}\n*/workspace-blocked.js\n127.0.0.1##.sponsored\n"
    ))?;
    let handle = app.clone();
    let fixture_state = state.clone();
    tauri::async_runtime::spawn(async move {
        let result = if overlay {
            run_overlay(&handle, &fixture_state, slots).await
        } else {
            host::show(&handle, slots, fixture_state.clone()).await
        };
        if let Err(error) = result {
            let _ = std::fs::write(fixture_state.data_dir.join("native-error.txt"), error);
            handle.exit(1);
        }
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Ok(runtime) = state.runtime.lock()
                && let Ok(json) = serde_json::to_string_pretty(&*runtime)
            {
                let _ = std::fs::write(state.data_dir.join("native-runtime.json"), json);
            }
            write_popup_snapshot(&state);
            poll_close_requests(&handle, &state);
            if state.data_dir.join("exit-request").exists() {
                handle.exit(0);
                break;
            }
        }
    });
    Ok(())
}

/// Per-service popup counts and activity, keyed by service name, for the fixture runner.
fn write_popup_snapshot(state: &AppState) {
    let (Ok(services), Ok(popups)) = (state.services.lock(), state.popups.lock()) else {
        return;
    };
    let mut report = serde_json::Map::new();
    for service in services.values() {
        let open: Vec<_> = popups
            .get(&service.id)
            .map(|entries| entries.values().collect())
            .unwrap_or_default();
        report.insert(
            service.name.clone(),
            serde_json::json!({ "count": open.len(), "popups": open }),
        );
    }
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = std::fs::write(state.data_dir.join("native-popups.json"), json);
    }
}

/// Trigger files `unload-A`, `remove-B` (and so on) run a close and write `<name>.result`.
fn poll_close_requests(handle: &tauri::AppHandle, state: &AppState) {
    for (tag, reason) in [
        ("unload", host::CloseReason::User),
        ("remove", host::CloseReason::Remove),
    ] {
        for role in ["A", "B"] {
            let trigger = state.data_dir.join(format!("{tag}-{role}"));
            if !trigger.exists() {
                continue;
            }
            let _ = std::fs::remove_file(&trigger);
            let id = state.services.lock().ok().and_then(|services| {
                services
                    .values()
                    .find(|s| s.name == format!("Fixture {role}"))
                    .map(|s| s.id.clone())
            });
            let Some(id) = id else { continue };
            let (handle, state) = (handle.clone(), state.clone());
            let result_path = state.data_dir.join(format!("{tag}-{role}.result"));
            tauri::async_runtime::spawn(async move {
                let outcome = match host::close(&handle, id, state, reason).await {
                    Ok(()) => "ok".to_string(),
                    Err(error) => error,
                };
                let _ = std::fs::write(result_path, outcome);
            });
        }
    }
}

#[cfg(not(windows))]
async fn run_overlay(_: &tauri::AppHandle, _: &AppState, _: Vec<ViewLayout>) -> Result<()> {
    Err("The overlay fixture needs Windows.".into())
}

/// Applies a series of surface revisions and hit-tests real screen points after each one.
#[cfg(windows)]
async fn run_overlay(
    app: &tauri::AppHandle,
    state: &AppState,
    slots: Vec<ViewLayout>,
) -> Result<()> {
    use workspace_core::surface::{LogicalRect, ServiceBounds, SurfaceLayout};
    let window = app
        .get_webview_window("main")
        .ok_or("Missing shell window.")?;
    window.set_focus().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(300))
    })
    .await
    .map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let id = slots
        .first()
        .ok_or("Missing fixture service.")?
        .service_id
        .clone();
    let layout = |revision: u64, hole: bool, capture: bool| SurfaceLayout {
        revision,
        services: vec![ServiceBounds {
            service_id: id.clone(),
            x: 20.0,
            y: 100.0,
            width: 500.0,
            height: 600.0,
        }],
        exclusions: if hole {
            vec![LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 170.0,
                height: 700.0,
            }]
        } else {
            vec![]
        },
        modal_input_capture: capture,
    };
    // Point 0 is inside the exclusion; point 1 is outside it. Both are inside the service view.
    let points = vec![
        ((50.0 * scale) as i32, (300.0 * scale) as i32),
        ((300.0 * scale) as i32, (300.0 * scale) as i32),
    ];
    let steps = [
        ("hole", layout(1, true, false)),
        ("no-hole", layout(2, false, false)),
        ("modal-capture", layout(3, true, true)),
        ("capture-released", layout(4, false, false)),
        // Revision 2 is older than the applied revision 4, so this must be ignored.
        ("stale-ignored", layout(2, true, false)),
    ];
    let mut report = Vec::new();
    for (name, step) in steps {
        host::show_surface(app, step, state.clone()).await?;
        tauri::async_runtime::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(600))
        })
        .await
        .map_err(|e| e.to_string())?;
        let probe = host::probe(app, id.clone(), points.clone()).await?;
        report.push(serde_json::json!({ "step": name, "probe": probe }));
    }
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(state.data_dir.join("native-overlay.json"), json).map_err(|e| e.to_string())
}
