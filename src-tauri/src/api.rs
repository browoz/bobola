//! Typed internal workspace boundary. The shell UI uses it today; a future adapter can reuse it.
//! It carries no transport, listener or automation of its own.
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use workspace_core::{
    Result, interface::InterfacePreferences, lifecycle::Runtime, surface::SurfaceLayout,
};

/// Single Tauri event channel for everything the native host tells the shell.
pub const EVENT_CHANNEL: &str = "workspace-event";

/// How long a burst of runtime changes is gathered before one `runtimeChanged` event is sent.
const RUNTIME_FLUSH_DELAY: Duration = Duration::from_millis(250);
/// True while a flush is scheduled. Changes arriving meanwhile ride on that single flush.
static RUNTIME_FLUSH_PENDING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorkspaceEvent {
    /// Ctrl+K was pressed while a service view had focus.
    QuickSwitchRequested {
        service_id: Option<String>,
    },
    PopupOpened {
        service_id: String,
        popup_id: u64,
    },
    PopupClosed {
        service_id: String,
        popup_id: u64,
    },
    /// A popup could not be created. `message` is the actual failure.
    PopupError {
        service_id: String,
        message: String,
    },
    /// Services, workspaces or legacy preferences changed. Payload is just `{"type":"snapshotChanged"}`;
    /// the UI answers by calling `bootstrap`.
    SnapshotChanged,
    /// Interface preferences were saved or restored. Carries the stored value.
    InterfacePreferencesChanged {
        preferences: InterfacePreferences,
    },
    /// Full runtime map as `bootstrap` reports it. Sent by `runtime_changed`, coalesced.
    RuntimeChanged {
        runtime: BTreeMap<String, Runtime>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorkspaceCommand {
    ApplySurface {
        layout: SurfaceLayout,
    },
    /// `None` restores focus to the last focused visible service.
    FocusService {
        service_id: Option<String>,
    },
    ClosePopups {
        service_id: String,
    },
}

pub fn emit(app: &tauri::AppHandle, event: WorkspaceEvent) {
    let _ = tauri::Emitter::emit(app, EVENT_CHANNEL, event);
}

/// Call after a successful state mutation, once every mutex guard is released.
pub fn snapshot_changed(app: &tauri::AppHandle) {
    emit(app, WorkspaceEvent::SnapshotChanged);
}

/// Call after interface preferences were stored, with no mutex guard held.
pub fn interface_preferences_changed(app: &tauri::AppHandle, preferences: InterfacePreferences) {
    emit(
        app,
        WorkspaceEvent::InterfacePreferencesChanged { preferences },
    );
}

/// Reports that runtime state (loaded, visible, blocked count, title, dirty/media) changed.
/// Cheap to call often: the first call in a burst schedules one flush after
/// `RUNTIME_FLUSH_DELAY`, later calls are absorbed by it, and nothing runs while idle.
/// The flush snapshots the runtime map after the delay and emits with no lock held.
/// Never call this while holding a state mutex.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn runtime_changed(app: &tauri::AppHandle, state: &AppState) {
    if RUNTIME_FLUSH_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    let state = state.clone();
    let spawned = std::thread::Builder::new()
        .name("runtime-flush".into())
        .spawn(move || {
            std::thread::sleep(RUNTIME_FLUSH_DELAY);
            // Cleared before reading, so a change landing after the read schedules a new flush.
            RUNTIME_FLUSH_PENDING.store(false, Ordering::Release);
            let runtime = match state.runtime.lock() {
                Ok(runtime) => runtime.clone(),
                Err(_) => return,
            };
            emit(&app, WorkspaceEvent::RuntimeChanged { runtime });
        });
    if spawned.is_err() {
        RUNTIME_FLUSH_PENDING.store(false, Ordering::Release);
    }
}

pub async fn dispatch(
    app: &tauri::AppHandle,
    state: AppState,
    command: WorkspaceCommand,
) -> Result<()> {
    match command {
        WorkspaceCommand::ApplySurface { layout } => {
            crate::host::show_surface(app, layout, state).await
        }
        WorkspaceCommand::FocusService { service_id } => {
            crate::host::focus_service(app, service_id, state).await
        }
        WorkspaceCommand::ClosePopups { service_id } => {
            crate::host::close_popups(app, service_id, state).await
        }
    }
}
