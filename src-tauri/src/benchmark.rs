//! Opt-in measurement harness. Not compiled into the distributed alpha.
use crate::{AppState, ViewLayout, host};
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
    time::Instant,
};
use workspace_core::{Result, model::NewService};

#[derive(Clone, Default, serde::Serialize)]
struct Traffic {
    requests: u64,
    blocked: u64,
    by_resource: BTreeMap<String, u64>,
    blocked_by_resource: BTreeMap<String, u64>,
    blocked_domains: BTreeMap<String, u64>,
}

fn traffic_map() -> &'static Mutex<BTreeMap<String, Traffic>> {
    static TRAFFIC: OnceLock<Mutex<BTreeMap<String, Traffic>>> = OnceLock::new();
    TRAFFIC.get_or_init(Default::default)
}

fn navigation_map() -> &'static Mutex<BTreeMap<String, u64>> {
    static NAVIGATIONS: OnceLock<Mutex<BTreeMap<String, u64>>> = OnceLock::new();
    NAVIGATIONS.get_or_init(Default::default)
}

fn started() -> &'static Instant {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    STARTED.get_or_init(Instant::now)
}

pub fn record_request(account: &str, raw_url: &str, kind: &str, blocked: bool) {
    let domain = url::Url::parse(raw_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_else(|| "unknown".into());
    if let Ok(mut traffic) = traffic_map().lock() {
        let entry = traffic.entry(account.into()).or_default();
        entry.requests += 1;
        *entry.by_resource.entry(kind.into()).or_default() += 1;
        if blocked {
            entry.blocked += 1;
            *entry.blocked_by_resource.entry(kind.into()).or_default() += 1;
            *entry.blocked_domains.entry(domain).or_default() += 1;
        }
    }
}

pub fn record_navigation(account: &str) {
    if let Ok(mut navigations) = navigation_map().lock() {
        navigations
            .entry(account.into())
            .or_insert_with(|| started().elapsed().as_millis() as u64);
    }
}

fn traffic_snapshot() -> BTreeMap<String, Traffic> {
    traffic_map()
        .lock()
        .map(|traffic| traffic.clone())
        .unwrap_or_default()
}

fn navigation_snapshot() -> BTreeMap<String, u64> {
    navigation_map()
        .lock()
        .map(|navigations| navigations.clone())
        .unwrap_or_default()
}

pub fn install(app: &tauri::AppHandle, state: AppState) -> Result<()> {
    let _ = started();
    let configured_urls: Option<Vec<(String, String)>> =
        std::env::var("SERVICE_WORKSPACE_BENCHMARK_URLS")
            .ok()
            .map(|raw| serde_json::from_str(&raw).map_err(|e| e.to_string()))
            .transpose()?;
    let (urls, origin) = if let Some(urls) = configured_urls {
        if urls.is_empty() || urls.len() > 20 {
            return Err("Benchmark URL count must be from 1 to 20.".into());
        }
        for (name, raw) in &urls {
            if name.trim().is_empty() || name.len() > 80 {
                return Err("Invalid benchmark service name.".into());
            }
            let parsed = url::Url::parse(raw).map_err(|e| e.to_string())?;
            if parsed.scheme() != "https" || parsed.host_str().is_none() {
                return Err("Real-page benchmark URLs must use HTTPS.".into());
            }
        }
        (urls, None)
    } else {
        let origin =
            std::env::var("SERVICE_WORKSPACE_BENCHMARK_ORIGIN").map_err(|e| e.to_string())?;
        let parsed = url::Url::parse(&origin).map_err(|e| e.to_string())?;
        if parsed.scheme() != "http" || parsed.host_str() != Some("127.0.0.1") {
            return Err("Synthetic benchmarks must use an owned loopback fixture.".into());
        }
        let count: usize = std::env::var("SERVICE_WORKSPACE_BENCHMARK_COUNT")
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|_| "Invalid fixture count".to_string())?;
        if count > 20 {
            return Err("Fixture count exceeds the benchmark limit.".into());
        }
        (
            (0..count)
                .map(|index| {
                    (
                        format!("Benchmark {}", index + 1),
                        format!("{origin}/fixture?account={index}"),
                    )
                })
                .collect(),
            Some(origin),
        )
    };
    let protection_enabled =
        match std::env::var("SERVICE_WORKSPACE_BENCHMARK_PROTECTION").as_deref() {
            Ok("on") => true,
            Ok("off") => false,
            Ok(_) => return Err("Protection benchmark mode must be on or off.".into()),
            Err(_) => true,
        };
    let mut slots = Vec::new();
    {
        let mut store = state.store.lock().map_err(|e| e.to_string())?;
        if let Some(file) = std::env::var_os("SERVICE_WORKSPACE_BENCHMARK_FILTER_FILE") {
            let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let engine = store.apply_filters(&text)?;
            *state.protection.lock().map_err(|e| e.to_string())? = engine;
        }
        let existing = store.snapshot()?.services;
        if !existing.is_empty() && existing.len() != urls.len() {
            return Err("Benchmark profile does not match the configured service list.".into());
        }
        for (local_index, (name, url)) in urls.iter().enumerate() {
            let mut service = if existing.is_empty() {
                store.add_service(NewService {
                    name: name.clone(),
                    url: url.clone(),
                    workspace_id: "personal".into(),
                    keep_live: Some(true),
                    catalog_id: None,
                })?
            } else {
                let service = &existing[local_index];
                if service.name != *name || service.url != *url {
                    return Err(
                        "Benchmark profile service list differs from configured URLs.".into(),
                    );
                }
                service.clone()
            };
            service.protection = protection_enabled;
            store.save_service(&service)?;
            let (x, y, width, height) = if origin.is_none() {
                // Ferdium keeps all service webviews mounted at the full service-area size,
                // with only the active view above the others. Match that geometry here.
                (0.0, 0.0, 1200.0, 760.0)
            } else {
                let columns = if urls.len() > 2 { 2 } else { urls.len().max(1) };
                let rows = urls.len().div_ceil(columns);
                (
                    (local_index % columns) as f64 * (1200.0 / columns as f64),
                    (local_index / columns) as f64 * (760.0 / rows as f64),
                    1200.0 / columns as f64,
                    760.0 / rows as f64,
                )
            };
            slots.push(ViewLayout {
                service_id: service.id,
                x,
                y,
                width,
                height,
            });
        }
    }
    crate::sync_services(&state)?;
    let handle = app.clone();
    let owned = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = host::show(&handle, slots, owned.clone()).await {
            let _ = std::fs::write(owned.data_dir.join("benchmark-error.txt"), error);
            handle.exit(1);
        }
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let traffic = traffic_snapshot();
            let navigation_completed_ms = navigation_snapshot();
            if let Ok(runtime) = state.runtime.lock() {
                let loaded = runtime.values().filter(|entry| entry.loaded).count();
                let info = serde_json::json!({
                    "root": state.data_dir.as_ref(),
                    "configured": urls.len(),
                    "loaded": loaded,
                    "protectionEnabled": protection_enabled,
                    "realPages": origin.is_none(),
                    "runtime": &*runtime,
                    "traffic": traffic,
                    "navigationCompletedMs": navigation_completed_ms,
                });
                let _ = std::fs::write(
                    state.data_dir.join("benchmark-state.json"),
                    info.to_string(),
                );
            }
            if state.data_dir.join("exit-request").exists() {
                handle.exit(0);
                break;
            }
        }
    });
    Ok(())
}
