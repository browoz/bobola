//! Window-region clipping of the WRY container HWNDs. This is what lets shell UI (a floating sidebar,
//! a modal) show through and receive input over a service view without resizing the website viewport.
//! All functions must run on the UI thread.
#[cfg(debug_assertions)]
use windows::Win32::Foundation::RECT;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
        CombineRgn, CreateRectRgn, DeleteObject, HGDIOBJ, HRGN, RGN_DIFF, SetWindowRgn,
    },
    UI::{
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::{HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos},
    },
};
use workspace_core::{Result, surface::SlotPlan};
use wry::{WebView, WebViewExtWindows};

/// Owns a GDI region until the system takes it over via `SetWindowRgn`.
pub struct Region(Option<HRGN>);

impl Region {
    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Result<Self> {
        let raw = unsafe { CreateRectRgn(left, top, right, bottom) };
        if raw.is_invalid() {
            return Err("Could not allocate a display region.".into());
        }
        Ok(Self(Some(raw)))
    }
    fn raw(&self) -> HRGN {
        self.0.unwrap_or_default()
    }
    /// Hands ownership to the caller; the region will no longer be freed on drop.
    fn release(mut self) -> HRGN {
        self.0.take().unwrap_or_default()
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        if let Some(raw) = self.0.take() {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(raw.0));
            }
        }
    }
}

/// Builds the region for one planned slot before any window is touched.
/// `None` means the view is fully visible and interactive.
pub fn prepare(plan: &SlotPlan) -> Result<Option<Region>> {
    if plan.captured {
        // An empty region paints nothing and receives no mouse input.
        return Region::rect(0, 0, 0, 0).map(Some);
    }
    if plan.holes.is_empty() {
        return Ok(None);
    }
    let base = Region::rect(0, 0, plan.bounds.width, plan.bounds.height)?;
    for hole in &plan.holes {
        let cut = Region::rect(hole.x, hole.y, hole.x + hole.width, hole.y + hole.height)?;
        // RGN_DIFF with the destination equal to the first source is allowed by GDI.
        let kind = unsafe {
            CombineRgn(
                Some(base.raw()),
                Some(base.raw()),
                Some(cut.raw()),
                RGN_DIFF,
            )
        };
        if kind.0 == 0 {
            return Err("Could not combine display regions.".into());
        }
    }
    Ok(Some(base))
}

pub fn container(view: &WebView) -> Result<HWND> {
    let mut hwnd = HWND::default();
    unsafe { view.controller().ParentWindow(&mut hwnd) }.map_err(|e| e.to_string())?;
    if hwnd.is_invalid() {
        return Err("The service view has no native container.".into());
    }
    Ok(hwnd)
}

/// Applies a prepared region and input state. After a successful `SetWindowRgn` the system owns the
/// region; on failure it is still ours and is freed here.
pub fn apply(view: &WebView, region: Option<Region>, captured: bool) -> Result<()> {
    let hwnd = container(view)?;
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    match region {
        Some(region) => {
            let raw = region.release();
            if unsafe { SetWindowRgn(hwnd, Some(raw), true) } == 0 {
                unsafe {
                    let _ = DeleteObject(HGDIOBJ(raw.0));
                }
                return Err("Could not clip the service view.".into());
            }
        }
        None => {
            unsafe { SetWindowRgn(hwnd, None, true) };
        }
    }
    // A disabled container also drops keyboard focus and tab navigation while a shell modal is open.
    unsafe {
        let _ = EnableWindow(hwnd, !captured);
    }
    Ok(())
}

/// Restores a full, interactive view.
pub fn reset(view: &WebView) -> Result<()> {
    apply(view, None, false)
}

#[cfg(debug_assertions)]
pub fn screen_rect(view: &WebView) -> Result<RECT> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut rect = RECT::default();
    unsafe { GetWindowRect(container(view)?, &mut rect) }.map_err(|e| e.to_string())?;
    Ok(rect)
}

/// Debug fixture helper: does a screen point land inside this view's container tree?
#[cfg(debug_assertions)]
pub fn hits_view(view: &WebView, x: i32, y: i32) -> Result<bool> {
    use windows::Win32::{
        Foundation::POINT,
        UI::WindowsAndMessaging::{IsChild, WindowFromPoint},
    };
    let hwnd = container(view)?;
    let found = unsafe { WindowFromPoint(POINT { x, y }) };
    let is_child = unsafe { IsChild(hwnd, found) }.as_bool();
    Ok(found == hwnd || is_child)
}
