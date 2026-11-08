//! Account-aware popups: a small owned Win32 top-level window hosting one WRY child view that is
//! created from the opener's WebView2 environment and handed back via `NewWindowResponse::Create`.
//! No Tauri window or webview is created, so the popup has no Tauri bridge.
//! Everything here runs on the UI thread. The registry borrow is never held across a native call.
use crate::{
    AppState,
    api::{self, WorkspaceEvent},
    host::{Role, attach, com_text},
    now,
};
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::{cell::RefCell, collections::BTreeMap, num::NonZeroIsize, rc::Rc, sync::OnceLock};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::ICoreWebView2, SourceChangedEventHandler,
    WindowCloseRequestedEventHandler,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
            MonitorFromWindow,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
    core::{HSTRING, PCWSTR, w},
};
use workspace_core::{
    Result,
    popup::{PopupGate, PopupState, popup_target, popup_title},
};
use wry::{
    NewWindowFeatures, NewWindowResponse, Rect, WebView, WebViewBuilder, WebViewBuilderExtWindows,
    WebViewExtWindows,
    dpi::{PhysicalPosition, PhysicalSize},
};

struct PopupWindow(HWND);

impl HasWindowHandle for PopupWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let raw = NonZeroIsize::new(self.0.0 as isize).ok_or(HandleError::Unavailable)?;
        let handle = Win32WindowHandle::new(raw);
        // The HWND outlives the builder call; the caller owns and destroys it.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

struct Popup {
    id: u64,
    account: String,
    view: WebView,
    app: tauri::AppHandle,
    state: AppState,
}

#[derive(Default)]
struct Registry {
    gate: PopupGate,
    next_id: u64,
    open: BTreeMap<isize, Rc<Popup>>,
}

thread_local! { static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default()); }

fn key(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn class_name() -> PCWSTR {
    w!("ServiceWorkspacePopup")
}

fn client_bounds(hwnd: HWND) -> Rect {
    let mut rect = RECT::default();
    let _ = unsafe { GetClientRect(hwnd, &mut rect) };
    Rect {
        position: PhysicalPosition::new(0i32, 0i32).into(),
        size: PhysicalSize::new(
            (rect.right - rect.left).max(1) as u32,
            (rect.bottom - rect.top).max(1) as u32,
        )
        .into(),
    }
}

fn fit(hwnd: HWND, view: &WebView) {
    let _ = view.set_bounds(client_bounds(hwnd));
}

fn set_title(hwnd: HWND, title: &str) {
    let _ = unsafe { SetWindowTextW(hwnd, &HSTRING::from(title)) };
}

fn lookup(hwnd: HWND) -> Option<Rc<Popup>> {
    REGISTRY
        .try_with(|registry| registry.try_borrow().ok()?.open.get(&key(hwnd)).cloned())
        .ok()
        .flatten()
}

/// Final cleanup, run from WM_DESTROY for every close path (user, JS `window.close`, account removal).
fn finish(hwnd: HWND) {
    let popup = REGISTRY
        .try_with(|registry| registry.try_borrow_mut().ok()?.open.remove(&key(hwnd)))
        .ok()
        .flatten();
    let Some(popup) = popup else { return };
    let _ = REGISTRY.try_with(|registry| {
        if let Ok(mut registry) = registry.try_borrow_mut() {
            registry.gate.release(&popup.account);
        }
    });
    if let Ok(mut table) = popup.state.popups.lock()
        && let Some(open) = table.get_mut(&popup.account)
    {
        open.remove(&popup.id);
        if open.is_empty() {
            table.remove(&popup.account);
        }
    }
    api::emit(
        &popup.app,
        WorkspaceEvent::PopupClosed {
            service_id: popup.account.clone(),
            popup_id: popup.id,
        },
    );
    // The WRY view is dropped here, after the registry borrow ended and outside any WebView2 handler.
    drop(popup);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SIZE => {
            if let Some(popup) = lookup(hwnd) {
                fit(hwnd, &popup.view);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            // lparam points at the system-suggested rectangle for the new monitor DPI.
            let suggested = unsafe { *(lparam.0 as *const RECT) };
            // Cap the suggested rectangle to the new monitor's work area. Resizing stays native.
            let mut placed = (
                suggested.left,
                suggested.top,
                suggested.right - suggested.left,
                suggested.bottom - suggested.top,
            );
            let monitor = unsafe { MonitorFromRect(&suggested, MONITOR_DEFAULTTONEAREST) };
            if let Some(work) = work_area(monitor) {
                placed = fit_to_work_area(placed.0, placed.1, placed.2, placed.3, work);
            }
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    placed.0,
                    placed.1,
                    placed.2,
                    placed.3,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            finish(hwnd);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
fn register_class() -> Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| unsafe {
            let module = GetModuleHandleW(None).map_err(|e| e.to_string())?;
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(window_proc),
                hInstance: module.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?,
                lpszClassName: class_name(),
                ..Default::default()
            };
            if RegisterClassExW(&class) == 0 {
                return Err("Could not register the popup window class.".to_string());
            }
            Ok(())
        })
        .clone()
}

const NOMINAL_WIDTH: i32 = 900;
const NOMINAL_HEIGHT: i32 = 700;

fn work_area(monitor: HMONITOR) -> Option<RECT> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info) }
        .as_bool()
        .then_some(info.rcWork)
}

/// Caps an outer window rectangle to a monitor work area and moves it so it lies fully inside.
fn fit_to_work_area(
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    work: RECT,
) -> (i32, i32, i32, i32) {
    let width = width.min(work.right - work.left).max(1);
    let height = height.min(work.bottom - work.top).max(1);
    (
        left.min(work.right - width).max(work.left),
        top.min(work.bottom - height).max(work.top),
        width,
        height,
    )
}

/// Scales the nominal 96-DPI size to the window's monitor and keeps it inside that work area.
fn place_initial(hwnd: HWND) {
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as i32;
    let mut rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
    let mut placed = (
        rect.left,
        rect.top,
        NOMINAL_WIDTH * dpi / 96,
        NOMINAL_HEIGHT * dpi / 96,
    );
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if let Some(work) = work_area(monitor) {
        placed = fit_to_work_area(placed.0, placed.1, placed.2, placed.3, work);
    }
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            None,
            placed.0,
            placed.1,
            placed.2,
            placed.3,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

fn create_window(title: &str) -> Result<HWND> {
    register_class()?;
    let module = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name(),
            &HSTRING::from(title),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            NOMINAL_WIDTH,
            NOMINAL_HEIGHT,
            None,
            None,
            Some(module.into()),
            None,
        )
    }
    .map_err(|e| format!("Could not create the popup window: {e}"))?;
    // The window is still hidden, so sizing it for the monitor it landed on causes no flicker.
    place_initial(hwnd);
    Ok(hwnd)
}

/// Releases a gate slot unless the popup was committed.
struct Reservation {
    account: String,
    armed: bool,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if self.armed {
            let _ = REGISTRY.try_with(|registry| {
                if let Ok(mut registry) = registry.try_borrow_mut() {
                    registry.gate.release(&self.account);
                }
            });
        }
    }
}

/// Destroys the native window unless the popup was committed.
struct WindowGuard {
    hwnd: HWND,
    armed: bool,
}
impl Drop for WindowGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = unsafe { DestroyWindow(self.hwnd) };
        }
    }
}

fn open(
    app: &tauri::AppHandle,
    state: &AppState,
    account: &str,
    url: &str,
    features: NewWindowFeatures,
) -> Result<ICoreWebView2> {
    popup_target(url)?;
    let service = state
        .services
        .lock()
        .map_err(|e| e.to_string())?
        .get(account)
        .cloned()
        .ok_or("Service not found.")?;
    // Short borrow: reserve a slot and an id, then release the registry before any native call.
    let id = REGISTRY.with(|registry| {
        let mut registry = registry
            .try_borrow_mut()
            .map_err(|_| "Another popup is still opening. Try again.".to_string())?;
        registry.gate.admit(account, now())?;
        registry.next_id += 1;
        Ok::<u64, String>(registry.next_id)
    })?;
    let mut reservation = Reservation {
        account: account.to_string(),
        armed: true,
    };
    let hwnd = create_window(&popup_title(&service.name, ""))?;
    let mut window = WindowGuard { hwnd, armed: true };
    // The target must stay un-navigated: no `with_url`. WebView2 navigates it after `SetNewWindow`.
    // The opener's environment and the account profile name are reused unchanged.
    let handle_app = app.clone();
    let handle_state = state.clone();
    let handle_account = account.to_string();
    let view = WebViewBuilder::new()
        .with_environment(features.opener.environment.clone())
        .with_profile_name(&service.profile_name)
        .with_bounds(client_bounds(hwnd))
        .with_initialization_script(include_str!("page-state.js"))
        .with_navigation_handler(|url| popup_target(&url).is_ok())
        .with_new_window_req_handler(new_window_handler(handle_app, handle_state, handle_account))
        .build_as_child(&PopupWindow(hwnd))
        .map_err(|e| format!("Could not create the popup view: {e}"))?;
    attach(&view, account, Role::Popup(id), app, state.clone())?;
    let core = view.webview();
    let title_name = service.name.clone();
    let on_source = SourceChangedEventHandler::create(Box::new(move |sender, _| {
        if let Some(sender) = sender {
            // Title comes from the browser-reported URL, never from page content.
            let source = com_text(|raw| unsafe { sender.Source(raw) }).unwrap_or_default();
            set_title(hwnd, &popup_title(&title_name, &source));
        }
        Ok(())
    }));
    let on_close = WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
        // HWNDs are recycled after destruction, so a late event must not close an unrelated window.
        // Popup ids are never reused; post only while this exact popup still owns the HWND.
        if lookup(hwnd).is_some_and(|open| open.id == id) {
            // Posted, not executed inline: the view is dropped later from WM_DESTROY.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        core.add_SourceChanged(&on_source, &mut token)
            .map_err(|e| e.to_string())?;
        core.add_WindowCloseRequested(&on_close, &mut token)
            .map_err(|e| e.to_string())?;
    }
    // Native creation can pump messages, so the account may have been removed meanwhile. Returning
    // here lets the guards drop the view, destroy the window and release the gate slot.
    if !state
        .services
        .lock()
        .map_err(|e| e.to_string())?
        .contains_key(account)
    {
        return Err("This service was removed while the popup was opening.".into());
    }
    let popup = Rc::new(Popup {
        id,
        account: account.to_string(),
        view,
        app: app.clone(),
        state: state.clone(),
    });
    REGISTRY.with(|registry| {
        registry
            .try_borrow_mut()
            .map_err(|_| "Popup registry is busy.".to_string())
            .map(|mut registry| registry.open.insert(key(hwnd), popup.clone()))
    })?;
    // From here WM_DESTROY owns cleanup (gate slot, state entry, view drop).
    reservation.armed = false;
    window.armed = false;
    if let Ok(mut table) = state.popups.lock() {
        table
            .entry(account.to_string())
            .or_default()
            .insert(id, PopupState::default());
    }
    api::emit(
        app,
        WorkspaceEvent::PopupOpened {
            service_id: account.to_string(),
            popup_id: id,
        },
    );
    fit(hwnd, &popup.view);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
    }
    Ok(core)
}

/// Handler for `window.open` / target=_blank. Real creation failures are reported, never a success notice.
pub fn new_window_handler(
    app: tauri::AppHandle,
    state: AppState,
    account: String,
) -> impl Fn(String, NewWindowFeatures) -> NewWindowResponse + 'static {
    move |url, features| {
        #[cfg(debug_assertions)]
        if std::env::var_os("SERVICE_WORKSPACE_TEST_OVERLAY").is_some() {
            return NewWindowResponse::Deny;
        }
        match open(&app, &state, &account, &url, features) {
            Ok(webview) => NewWindowResponse::Create { webview },
            Err(message) => {
                api::emit(
                    &app,
                    WorkspaceEvent::PopupError {
                        service_id: account.clone(),
                        message,
                    },
                );
                NewWindowResponse::Deny
            }
        }
    }
}

/// Closes every popup of an account. Called from account close/removal on the UI thread.
pub fn close_account(account: &str) {
    let targets: Vec<isize> = REGISTRY
        .try_with(|registry| {
            registry
                .try_borrow()
                .map(|registry| {
                    registry
                        .open
                        .iter()
                        .filter(|(_, popup)| popup.account == account)
                        .map(|(hwnd, _)| *hwnd)
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();
    for hwnd in targets {
        // The borrow above is released; WM_DESTROY re-enters `finish` safely.
        let _ = unsafe { DestroyWindow(HWND(hwnd as *mut core::ffi::c_void)) };
    }
}

pub fn clear_cosmetics(account: &str) {
    let popups = REGISTRY.with(|registry| {
        registry
            .borrow()
            .open
            .values()
            .filter(|popup| popup.account == account)
            .cloned()
            .collect::<Vec<_>>()
    });
    for popup in popups {
        let _ = popup
            .view
            .evaluate_script("document.getElementById('__workspace_protection')?.remove()");
    }
}
