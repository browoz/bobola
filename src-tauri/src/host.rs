#[derive(Clone, Copy, PartialEq)]
pub enum CloseReason {
    User,
    Idle,
    Remove,
}

#[cfg(windows)]
mod windows_host {
    use super::CloseReason;
    use crate::{
        AppState, ViewLayout,
        api::{self, WorkspaceEvent},
        native_popup, native_surface, now,
    };
    use serde::Deserialize;
    use std::{cell::RefCell, collections::BTreeMap};
    use tauri::Manager;
    use webview2_com::{
        AcceleratorKeyPressedEventHandler, FocusChangedEventHandler,
        Microsoft::Web::WebView2::Win32::*, NavigationCompletedEventHandler,
        WebMessageReceivedEventHandler, WebResourceRequestedEventHandler,
    };
    use windows::{
        Win32::{
            System::Com::CoTaskMemFree,
            UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT},
        },
        core::{HSTRING, Interface, PWSTR, w},
    };
    use workspace_core::{
        Result,
        lifecycle::Runtime,
        model::Service,
        popup::{PopupActivity, summarize, with_popups},
        surface::{self, SlotPlan, SurfaceLayout},
    };
    use wry::{
        Rect, WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows,
        dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    };

    #[derive(Default)]
    struct Host {
        context: Option<WebContext>,
        views: BTreeMap<String, WebView>,
        /// Highest surface layout revision applied so far.
        revision: u64,
    }
    thread_local! {
        static HOST: RefCell<Host> = RefCell::new(Host::default());
        // Kept apart from HOST so focus tracking never needs the host borrow.
        static LAST_FOCUSED: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Which view of an account a handler is attached to.
    #[derive(Clone, Copy)]
    pub enum Role {
        Main,
        Popup(u64),
    }

    /// Writes page activity to the main runtime entry or to the popup's own entry.
    /// Returns true when the main runtime entry changed, so the caller can report it
    /// after the lock is released. Popup activity is not part of the runtime map.
    fn record_activity(
        state: &AppState,
        account: &str,
        role: Role,
        dirty: bool,
        media: bool,
    ) -> bool {
        match role {
            Role::Main => {
                if let Ok(mut runtime) = state.runtime.lock() {
                    let existed = runtime.contains_key(account);
                    let entry = runtime.entry(account.to_string()).or_default();
                    let changed = !existed || entry.dirty != dirty || entry.media != media;
                    entry.dirty = dirty;
                    entry.media = media;
                    return changed;
                }
                false
            }
            Role::Popup(popup) => {
                if let Ok(mut popups) = state.popups.lock()
                    && let Some(entry) = popups.get_mut(account).and_then(|m| m.get_mut(&popup))
                {
                    entry.dirty = dirty;
                    entry.media = media;
                }
                false
            }
        }
    }

    fn popup_activity(state: &AppState, account: &str) -> Result<PopupActivity> {
        let popups = state.popups.lock().map_err(|e| e.to_string())?;
        Ok(popups
            .get(account)
            .map(|open| summarize(open.values()))
            .unwrap_or_default())
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PageMessage {
        kind: String,
        dirty: bool,
        media: bool,
        classes: Vec<String>,
        ids: Vec<String>,
        #[serde(default)]
        background: Option<[u8; 3]>,
    }

    pub(crate) fn com_text(
        read: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
    ) -> windows::core::Result<String> {
        let mut raw = PWSTR::null();
        read(&mut raw)?;
        if raw.is_null() {
            return Ok(String::new());
        }
        let value = unsafe { raw.to_string() };
        unsafe { CoTaskMemFree(Some(raw.0.cast())) };
        Ok(value?)
    }

    /// Installs the shared blocker, page-state bridge and lifecycle hooks on a view.
    /// Popups reuse the same blocker but report activity to their own entry.
    pub(crate) fn attach(
        view: &WebView,
        id: &str,
        role: Role,
        app: &tauri::AppHandle,
        state: AppState,
    ) -> Result<()> {
        let core = view.webview();
        let environment = view.environment();
        unsafe {
            if matches!(role, Role::Main) {
                let mut pid = 0u32;
                core.BrowserProcessId(&mut pid).map_err(|e| e.to_string())?;
                if let Ok(mut runtime) = state.runtime.lock() {
                    runtime.entry(id.to_string()).or_default().browser_pid = Some(pid);
                }
            }
            let mut token = 0;
            if let Ok(extended) = core.cast::<ICoreWebView2_22>() {
                extended
                    .AddWebResourceRequestedFilterWithRequestSourceKinds(
                        w!("*"),
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )
                    .map_err(|e| e.to_string())?;
            } else {
                core.AddWebResourceRequestedFilter(w!("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
                    .map_err(|e| e.to_string())?;
            }
            let account = id.to_string();
            let network_state = state.clone();
            let network_app = app.clone();
            let handler =
                WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                    let (Some(sender), Some(args)) = (sender, args) else {
                        return Ok(());
                    };
                    let request = args.Request()?;
                    let url = com_text(|raw| request.Uri(raw))?;
                    let method =
                        com_text(|raw| request.Method(raw)).unwrap_or_else(|_| "GET".into());
                    let enabled = network_state
                        .services
                        .lock()
                        .ok()
                        .and_then(|services| {
                            services.get(&account).map(|service| service.protection)
                        })
                        .unwrap_or(false);
                    let source = com_text(|raw| request.Headers()?.GetHeader(w!("Referer"), raw))
                        .or_else(|_| com_text(|raw| sender.Source(raw)))
                        .unwrap_or_default();
                    let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                    args.ResourceContext(&mut context)?;
                    let kind = match context {
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST
                        | COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH => "xmlhttprequest",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => "document",
                        _ => "other",
                    };
                    let blocked = network_state
                        .protection
                        .lock()
                        .map(|engine| {
                            engine
                                .check_method(&url, &source, kind, &method, enabled)
                                .blocked
                        })
                        .unwrap_or(false);
                    #[cfg(feature = "benchmark-harness")]
                    crate::benchmark::record_request(&account, &url, kind, blocked);
                    if blocked {
                        let response = environment.CreateWebResourceResponse(
                            None::<&windows::Win32::System::Com::IStream>,
                            403,
                            w!("Blocked by Bobola"),
                            w!("Content-Type: text/plain\r\nCache-Control: no-store"),
                        )?;
                        args.SetResponse(&response)?;
                        if let Ok(mut runtime) = network_state.runtime.lock() {
                            runtime.entry(account.clone()).or_default().blocked += 1;
                        }
                        // Blocked requests arrive in bursts; the coalescer sends one event per burst.
                        api::runtime_changed(&network_app, &network_state);
                    }
                    Ok(())
                }));
            core.add_WebResourceRequested(&handler, &mut token)
                .map_err(|e| e.to_string())?;

            core.Settings()
                .and_then(|settings| settings.SetIsWebMessageEnabled(true))
                .map_err(|e| e.to_string())?;
            let account = id.to_string();
            let message_state = state.clone();
            let message_app = app.clone();
            let handler = WebMessageReceivedEventHandler::create(Box::new(move |sender, args| {
                let (Some(sender), Some(args)) = (sender, args) else {
                    return Ok(());
                };
                let text = com_text(|raw| args.TryGetWebMessageAsString(raw))?;
                if text.len() > 65536 {
                    return Ok(());
                }
                let Ok(message) = serde_json::from_str::<PageMessage>(&text) else {
                    return Ok(());
                };
                if message.kind != "page-state"
                    || message.classes.len() > 300
                    || message.ids.len() > 300
                    || message
                        .classes
                        .iter()
                        .chain(&message.ids)
                        .any(|part| part.len() > 128)
                {
                    return Ok(());
                }
                let source = com_text(|raw| args.Source(raw))?;
                let current = com_text(|raw| sender.Source(raw))?;
                // This bridge accepts main-document state only and has no file, process or settings operations.
                if source != current {
                    return Ok(());
                }
                let mut changed =
                    record_activity(&message_state, &account, role, message.dirty, message.media);
                if matches!(role, Role::Main)
                    && let Ok(mut runtime) = message_state.runtime.lock()
                {
                    let entry = runtime.entry(account.clone()).or_default();
                    if entry.background != message.background {
                        entry.background = message.background;
                        changed = true;
                    }
                }
                if changed {
                    api::runtime_changed(&message_app, &message_state);
                }
                let enabled = message_state
                    .services
                    .lock()
                    .ok()
                    .and_then(|services| services.get(&account).map(|service| service.protection))
                    .unwrap_or(false);
                let css = if enabled {
                    message_state
                        .protection
                        .lock()
                        .map(|engine| engine.cosmetic_css(&current, &message.classes, &message.ids))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let css = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into());
                let script = format!(
                    "(()=>{{let s=document.getElementById('__workspace_protection');if(!s){{s=document.createElement('style');s.id='__workspace_protection';document.documentElement.append(s)}}const css={css};if(s.textContent!==css)s.textContent=css}})()"
                );
                sender.ExecuteScript(&HSTRING::from(script), None)?;
                Ok(())
            }));
            core.add_WebMessageReceived(&handler, &mut token)
                .map_err(|e| e.to_string())?;

            let account = id.to_string();
            let nav_state = state.clone();
            let nav_app = app.clone();
            let handler = NavigationCompletedEventHandler::create(Box::new(move |_, _| {
                #[cfg(feature = "benchmark-harness")]
                crate::benchmark::record_navigation(&account);
                if record_activity(&nav_state, &account, role, false, false) {
                    api::runtime_changed(&nav_app, &nav_state);
                }
                Ok(())
            }));
            core.add_NavigationCompleted(&handler, &mut token)
                .map_err(|e| e.to_string())?;

            if matches!(role, Role::Main) {
                let account = id.to_string();
                let handler = FocusChangedEventHandler::create(Box::new(move |_, _| {
                    LAST_FOCUSED.with(|last| *last.borrow_mut() = Some(account.clone()));
                    Ok(())
                }));
                if let Ok(controller) = view.controller().cast::<ICoreWebView2Controller>() {
                    let mut focus_token = 0;
                    controller
                        .add_GotFocus(&handler, &mut focus_token)
                        .map_err(|e| e.to_string())?;
                    // Ctrl+K while a service has focus. Scoped to this view's controller,
                    // not an OS-global hook; every other key reaches the page unchanged.
                    let account = id.to_string();
                    let app = app.clone();
                    let accel =
                        AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                            let Some(args) = args else { return Ok(()) };
                            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                            args.KeyEventKind(&mut kind)?;
                            let mut key = 0u32;
                            args.VirtualKey(&mut key)?;
                            let down = |vk: u16| (GetKeyState(vk.into()) as u16 & 0x8000) != 0;
                            let exact = down(VK_CONTROL.0) && !down(VK_SHIFT.0) && !down(VK_MENU.0);
                            let is_down = kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                            if exact && key == u32::from(b'K') && is_down {
                                args.SetHandled(true)?;
                                let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
                                args.PhysicalKeyStatus(&mut status)?;
                                if !status.WasKeyDown.as_bool() {
                                    api::emit(
                                        &app,
                                        WorkspaceEvent::QuickSwitchRequested {
                                            service_id: Some(account.clone()),
                                        },
                                    );
                                    focus_shell(&app);
                                }
                            }
                            Ok(())
                        }));
                    controller
                        .add_AcceleratorKeyPressed(&accel, &mut focus_token)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    /// Moves keyboard focus to the shell so its quick-switch dialog can take input.
    fn focus_shell(app: &tauri::AppHandle) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_focus();
        }
    }

    async fn on_main<T: Send + 'static>(
        app: &tauri::AppHandle,
        operation: impl FnOnce() -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (send, receive) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let _ = send.send(operation());
        })
        .map_err(|e| e.to_string())?;
        receive
            .await
            .map_err(|_| "The native browser operation was interrupted.".to_string())?
    }

    /// Creates the account's main view on first use. The profile name is never altered.
    fn ensure_view(
        host: &mut Host,
        app: &tauri::AppHandle,
        parent: &tauri::WebviewWindow,
        service: &Service,
        bounds: Rect,
        state: &AppState,
    ) -> Result<()> {
        if host.views.contains_key(&service.id) {
            return Ok(());
        }
        if host.context.is_none() {
            let folder = state.data_dir.join("browser");
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            host.context = Some(WebContext::new(Some(folder)));
        }
        let title_state = state.clone();
        let title_app = app.clone();
        let title_id = service.id.clone();
        let context = host.context.as_mut().ok_or("Browser context is missing.")?;
        let view = WebViewBuilder::new_with_web_context(context)
            .with_profile_name(&service.profile_name)
            .with_bounds(bounds)
            .with_url("about:blank")
            .with_initialization_script(include_str!("page-state.js"))
            .with_navigation_handler(|url| {
                url == "about:blank" || workspace_core::model::service_url(&url).is_ok()
            })
            .with_document_title_changed_handler(move |title| {
                let changed = match title_state.runtime.lock() {
                    Ok(mut runtime) => {
                        let entry = runtime.entry(title_id.clone()).or_default();
                        let changed = entry.title != title;
                        entry.title = title;
                        changed
                    }
                    Err(_) => false,
                };
                if changed {
                    api::runtime_changed(&title_app, &title_state);
                }
            })
            .with_new_window_req_handler(native_popup::new_window_handler(
                app.clone(),
                state.clone(),
                service.id.clone(),
            ))
            .build_as_child(parent)
            .map_err(|e| e.to_string())?;
        attach(&view, &service.id, Role::Main, app, state.clone())?;
        let resume_url = state
            .runtime
            .lock()
            .ok()
            .and_then(|runtime| {
                runtime
                    .get(&service.id)
                    .and_then(|entry| entry.last_url.clone())
            })
            .filter(|url| workspace_core::model::service_url(url).is_ok());
        view.load_url(resume_url.as_deref().unwrap_or(&service.url))
            .map_err(|e| e.to_string())?;
        host.views.insert(service.id.clone(), view);
        Ok(())
    }

    pub async fn show(
        app: &tauri::AppHandle,
        layout: Vec<ViewLayout>,
        state: AppState,
    ) -> Result<()> {
        let notify = state.clone();
        let result = show_inner(app, layout, state).await;
        // Visibility and loaded flags may have changed even when a later slot failed.
        api::runtime_changed(app, &notify);
        result
    }

    async fn show_inner(
        app: &tauri::AppHandle,
        layout: Vec<ViewLayout>,
        state: AppState,
    ) -> Result<()> {
        let handle = app.clone();
        on_main(app, move || {
            HOST.with(|cell| {
                let mut host = cell
                    .try_borrow_mut()
                    .map_err(|_| "Another browser view is still opening. Try again.".to_string())?;
                let parent = handle
                    .get_webview_window("main")
                    .ok_or("Main window not found.")?;
                for (id, view) in &host.views {
                    let visible = layout.iter().any(|slot| slot.service_id == *id);
                    view.set_visible(visible).map_err(|e| e.to_string())?;
                    if let Ok(mut runtime) = state.runtime.lock() {
                        runtime.entry(id.clone()).or_default().visible = visible;
                    }
                }
                for slot in layout {
                    let service = state
                        .services
                        .lock()
                        .map_err(|e| e.to_string())?
                        .get(&slot.service_id)
                        .cloned()
                        .ok_or("Service not found.")?;
                    let bounds = Rect {
                        position: LogicalPosition::new(slot.x, slot.y).into(),
                        size: LogicalSize::new(slot.width, slot.height).into(),
                    };
                    ensure_view(&mut host, &handle, &parent, &service, bounds, &state)?;
                    let view = &host.views[&service.id];
                    view.set_bounds(bounds).map_err(|e| e.to_string())?;
                    view.set_visible(true).map_err(|e| e.to_string())?;
                    // The legacy layout path never leaves an overlay region or input capture behind.
                    native_surface::reset(view)?;
                    if let Ok(mut runtime) = state.runtime.lock() {
                        let entry = runtime
                            .entry(service.id.clone())
                            .or_insert_with(Runtime::default);
                        entry.loaded = true;
                        entry.visible = true;
                        entry.last_active = now();
                    }
                }
                Ok(())
            })
        })
        .await
    }

    fn physical(plan: &SlotPlan) -> Rect {
        Rect {
            position: PhysicalPosition::new(plan.bounds.x, plan.bounds.y).into(),
            size: PhysicalSize::new(
                plan.bounds.width.max(1) as u32,
                plan.bounds.height.max(1) as u32,
            )
            .into(),
        }
    }

    /// Applies every planned slot. Regions were built beforehand, so nothing here allocates GDI objects
    /// that could fail halfway; a failure resets every view to a full, interactive surface.
    fn apply_plans(
        host: &mut Host,
        app: &tauri::AppHandle,
        parent: &tauri::WebviewWindow,
        state: &AppState,
        plans: &[SlotPlan],
        services: &[Service],
        regions: Vec<Option<native_surface::Region>>,
    ) -> Result<()> {
        for (id, view) in &host.views {
            let visible = plans.iter().any(|plan| plan.service_id == *id);
            view.set_visible(visible).map_err(|e| e.to_string())?;
            if let Ok(mut runtime) = state.runtime.lock() {
                runtime.entry(id.clone()).or_default().visible = visible;
            }
        }
        for ((plan, service), region) in plans.iter().zip(services).zip(regions) {
            let bounds = physical(plan);
            ensure_view(host, app, parent, service, bounds, state)?;
            let view = &host.views[&service.id];
            view.set_bounds(bounds).map_err(|e| e.to_string())?;
            view.set_visible(true).map_err(|e| e.to_string())?;
            native_surface::apply(view, region, plan.captured)?;
            if let Ok(mut runtime) = state.runtime.lock() {
                let entry = runtime.entry(service.id.clone()).or_default();
                entry.loaded = true;
                entry.visible = true;
                entry.last_active = now();
            }
        }
        Ok(())
    }

    /// Atomic surface update: view bounds, overlay exclusions and modal capture in one UI-thread pass.
    /// A layout older than the last applied revision is ignored.
    pub async fn show_surface(
        app: &tauri::AppHandle,
        layout: SurfaceLayout,
        state: AppState,
    ) -> Result<()> {
        let notify = state.clone();
        let result = show_surface_inner(app, layout, state).await;
        api::runtime_changed(app, &notify);
        result
    }

    async fn show_surface_inner(
        app: &tauri::AppHandle,
        layout: SurfaceLayout,
        state: AppState,
    ) -> Result<()> {
        let handle = app.clone();
        on_main(app, move || {
            let parent = handle
                .get_webview_window("main")
                .ok_or("Main window not found.")?;
            let scale = parent.scale_factor().map_err(|e| e.to_string())?;
            let plans = surface::plan(&layout, scale)?;
            HOST.with(|cell| {
                let mut host = cell
                    .try_borrow_mut()
                    .map_err(|_| "Another browser view is still opening. Try again.".to_string())?;
                if surface::is_stale(host.revision, layout.revision) {
                    return Ok(());
                }
                let mut services = Vec::new();
                for plan in &plans {
                    services.push(
                        state
                            .services
                            .lock()
                            .map_err(|e| e.to_string())?
                            .get(&plan.service_id)
                            .cloned()
                            .ok_or("Service not found.")?,
                    );
                }
                let mut regions = Vec::new();
                for plan in &plans {
                    regions.push(native_surface::prepare(plan)?);
                }
                match apply_plans(
                    &mut host, &handle, &parent, &state, &plans, &services, regions,
                ) {
                    Ok(()) => {
                        host.revision = layout.revision;
                        Ok(())
                    }
                    Err(error) => {
                        for view in host.views.values() {
                            let _ = native_surface::reset(view);
                        }
                        Err(error)
                    }
                }
            })
        })
        .await
    }

    /// Returns keyboard focus to a visible service view (the last focused one by default).
    pub async fn focus_service(
        app: &tauri::AppHandle,
        id: Option<String>,
        state: AppState,
    ) -> Result<()> {
        on_main(app, move || {
            let target = id.or_else(|| LAST_FOCUSED.with(|last| last.borrow().clone()));
            let Some(target) = target else { return Ok(()) };
            let visible = state
                .runtime
                .lock()
                .map_err(|e| e.to_string())?
                .get(&target)
                .is_some_and(|entry| entry.visible);
            if !visible {
                return Ok(());
            }
            HOST.with(|cell| {
                let host = cell
                    .try_borrow()
                    .map_err(|_| "Another browser view is still opening. Try again.".to_string())?;
                if let Some(view) = host.views.get(&target) {
                    view.focus().map_err(|e| e.to_string())?;
                }
                Ok(())
            })
        })
        .await
    }

    /// Debug fixture result: container size in physical pixels and whether each probe point hits the view.
    #[cfg(debug_assertions)]
    #[derive(serde::Serialize)]
    pub struct Probe {
        pub width: i32,
        pub height: i32,
        pub hits: Vec<bool>,
    }

    /// Debug fixture helper. Points are physical pixels relative to the service view's top-left corner.
    #[cfg(debug_assertions)]
    pub async fn probe(
        app: &tauri::AppHandle,
        id: String,
        points: Vec<(i32, i32)>,
    ) -> Result<Probe> {
        on_main(app, move || {
            HOST.with(|cell| {
                let host = cell
                    .try_borrow()
                    .map_err(|_| "Another browser view is still opening. Try again.".to_string())?;
                let view = host.views.get(&id).ok_or("Service view is not loaded.")?;
                let rect = native_surface::screen_rect(view)?;
                let mut hits = Vec::new();
                for (x, y) in points {
                    hits.push(native_surface::hits_view(
                        view,
                        rect.left + x,
                        rect.top + y,
                    )?);
                }
                Ok(Probe {
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                    hits,
                })
            })
        })
        .await
    }

    pub async fn close_popups(app: &tauri::AppHandle, id: String, _state: AppState) -> Result<()> {
        on_main(app, move || {
            native_popup::close_account(&id);
            Ok(())
        })
        .await
    }

    pub async fn clear_cosmetics(app: &tauri::AppHandle, id: String) -> Result<()> {
        on_main(app, move || {
            let script = "document.getElementById('__workspace_protection')?.remove()";
            HOST.with(|cell| {
                if let Some(view) = cell.borrow().views.get(&id) {
                    view.evaluate_script(script).map_err(|e| e.to_string())?;
                }
                Ok::<(), String>(())
            })?;
            native_popup::clear_cosmetics(&id);
            Ok(())
        })
        .await
    }

    pub async fn browser_process_ids(app: &tauri::AppHandle) -> Result<Vec<u32>> {
        on_main(app, move || {
            HOST.with(|cell| {
                let host = cell.borrow();
                match host.views.values().next() {
                    Some(view) => crate::resources::environment_pids(&view.environment()),
                    None => Ok(Vec::new()),
                }
            })
        })
        .await
    }

    pub async fn close(
        app: &tauri::AppHandle,
        id: String,
        state: AppState,
        reason: CloseReason,
    ) -> Result<()> {
        let notify = state.clone();
        let result = close_inner(app, id, state, reason).await;
        // A refused close changes nothing, but one extra coalesced event is harmless.
        api::runtime_changed(app, &notify);
        result
    }

    async fn close_inner(
        app: &tauri::AppHandle,
        id: String,
        state: AppState,
        reason: CloseReason,
    ) -> Result<()> {
        on_main(app, move || {
            HOST.with(|cell| {
                // Re-check on the UI thread: a service may have become visible while eviction was queued.
                if reason == CloseReason::Idle
                    && state
                        .services
                        .lock()
                        .map_err(|e| e.to_string())?
                        .get(&id)
                        .is_some_and(|service| service.keep_live)
                {
                    return Err("This service is set to stay live.".into());
                }
                if reason != CloseReason::Remove {
                    // Main-view flags and popup activity are aggregated here without mixing them in storage.
                    let activity = popup_activity(&state, &id)?;
                    let runtime = state.runtime.lock().map_err(|e| e.to_string())?;
                    let merged =
                        with_popups(&runtime.get(&id).cloned().unwrap_or_default(), activity);
                    if merged.media || merged.dirty || merged.visible {
                        return Err(if activity.open > 0 {
                            "This account has a popup window open. Close it first.".into()
                        } else {
                            "This service is open, playing media, or may contain unsaved work."
                                .to_string()
                        });
                    }
                }
                if reason != CloseReason::Remove
                    && let Some(view) = cell.borrow().views.get(&id)
                    && let Ok(url) = view.url()
                    && workspace_core::model::service_url(&url).is_ok()
                    && let Ok(mut runtime) = state.runtime.lock()
                {
                    runtime.entry(id.clone()).or_default().last_url = Some(url);
                }
                // Removing an account closes its popups first, outside any host borrow.
                native_popup::close_account(&id);
                let removed = cell
                    .try_borrow_mut()
                    .map_err(|_| "Another browser view is still opening. Try again.".to_string())?
                    .views
                    .remove(&id);
                drop(removed);
                LAST_FOCUSED.with(|last| {
                    let mut last = last.borrow_mut();
                    if last.as_deref() == Some(id.as_str()) {
                        *last = None;
                    }
                });
                if let Ok(mut runtime) = state.runtime.lock()
                    && let Some(entry) = runtime.get_mut(&id)
                {
                    entry.loaded = false;
                    entry.visible = false;
                }
                Ok(())
            })
        })
        .await
    }
    pub async fn reload(app: &tauri::AppHandle, id: String) -> Result<()> {
        on_main(app, move || {
            HOST.with(|cell| {
                if let Some(view) = cell.borrow().views.get(&id) {
                    view.evaluate_script("location.reload()")
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            })
        })
        .await
    }
}

#[cfg(windows)]
pub use windows_host::*;

#[cfg(not(windows))]
pub async fn show(
    _: &tauri::AppHandle,
    _: Vec<crate::ViewLayout>,
    _: crate::AppState,
) -> workspace_core::Result<()> {
    Err("The native service host currently supports Windows.".into())
}
#[cfg(not(windows))]
pub async fn close(
    _: &tauri::AppHandle,
    _: String,
    _: crate::AppState,
    _: CloseReason,
) -> workspace_core::Result<()> {
    Ok(())
}
#[cfg(not(windows))]
pub async fn reload(_: &tauri::AppHandle, _: String) -> workspace_core::Result<()> {
    Ok(())
}
#[cfg(not(windows))]
pub async fn show_surface(
    _: &tauri::AppHandle,
    _: workspace_core::surface::SurfaceLayout,
    _: crate::AppState,
) -> workspace_core::Result<()> {
    Err("The native service host currently supports Windows.".into())
}
#[cfg(not(windows))]
pub async fn focus_service(
    _: &tauri::AppHandle,
    _: Option<String>,
    _: crate::AppState,
) -> workspace_core::Result<()> {
    Ok(())
}
#[cfg(not(windows))]
pub async fn close_popups(
    _: &tauri::AppHandle,
    _: String,
    _: crate::AppState,
) -> workspace_core::Result<()> {
    Ok(())
}
