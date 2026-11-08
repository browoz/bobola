//! Pure planning for the native surface layout: logical client pixels in, physical pixels out.
use crate::Result;
use serde::{Deserialize, Serialize};

pub const MAX_SERVICES: usize = 5;
pub const MAX_EXCLUSIONS: usize = 16;
const LIMIT: f64 = 20000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceBounds {
    pub service_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// One atomic description of the native surface. Coordinates are logical client pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceLayout {
    pub revision: u64,
    pub services: Vec<ServiceBounds>,
    #[serde(default)]
    pub exclusions: Vec<LogicalRect>,
    /// When true every service view stops receiving input and paints nothing, so a shell modal gets all input.
    #[serde(default)]
    pub modal_input_capture: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotPlan {
    pub service_id: String,
    pub bounds: PixelRect,
    /// Rectangles to cut out of the view, relative to the view's top-left corner.
    pub holes: Vec<PixelRect>,
    /// The whole view is cut out and ignores input.
    pub captured: bool,
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite() && v.abs() <= LIMIT)
}

/// A layout is stale when a newer revision has already been applied.
pub fn is_stale(applied: u64, incoming: u64) -> bool {
    incoming < applied
}

fn to_pixels(x: f64, y: f64, width: f64, height: f64, scale: f64) -> PixelRect {
    let left = (x * scale).round();
    let top = (y * scale).round();
    let right = ((x + width) * scale).round();
    let bottom = ((y + height) * scale).round();
    PixelRect {
        x: left as i32,
        y: top as i32,
        width: ((right - left) as i32).max(1),
        height: ((bottom - top) as i32).max(1),
    }
}

/// Like `to_pixels`, but grows to whole pixels: floor left/top, ceil right/bottom. Used for overlay
/// exclusions so a fractional-DPI edge never leaves native pixels showing over an overlay.
fn to_pixels_outward(x: f64, y: f64, width: f64, height: f64, scale: f64) -> PixelRect {
    let left = (x * scale).floor();
    let top = (y * scale).floor();
    let right = ((x + width) * scale).ceil();
    let bottom = ((y + height) * scale).ceil();
    PixelRect {
        x: left as i32,
        y: top as i32,
        width: ((right - left) as i32).max(1),
        height: ((bottom - top) as i32).max(1),
    }
}

pub fn validate(layout: &SurfaceLayout) -> Result<()> {
    if layout.services.len() > MAX_SERVICES || layout.exclusions.len() > MAX_EXCLUSIONS {
        return Err("Invalid surface layout.".into());
    }
    for (index, item) in layout.services.iter().enumerate() {
        let ok = finite(&[item.x, item.y, item.width, item.height])
            && item.x >= 0.0
            && item.y >= 0.0
            && item.width >= 1.0
            && item.height >= 1.0
            && !item.service_id.is_empty()
            && item.service_id.len() <= 64
            && !layout.services[..index]
                .iter()
                .any(|other| other.service_id == item.service_id);
        if !ok {
            return Err("Invalid surface layout.".into());
        }
    }
    if layout
        .exclusions
        .iter()
        .any(|r| !finite(&[r.x, r.y, r.width, r.height]) || r.width <= 0.0 || r.height <= 0.0)
    {
        return Err("Invalid surface exclusion.".into());
    }
    Ok(())
}

/// Converts a validated layout to physical pixels. `scale` is DPI / 96.
pub fn plan(layout: &SurfaceLayout, scale: f64) -> Result<Vec<SlotPlan>> {
    validate(layout)?;
    if !scale.is_finite() || !(0.5..=8.0).contains(&scale) {
        return Err("Invalid display scale.".into());
    }
    let exclusions: Vec<PixelRect> = layout
        .exclusions
        .iter()
        .map(|r| to_pixels_outward(r.x, r.y, r.width, r.height, scale))
        .collect();
    Ok(layout
        .services
        .iter()
        .map(|item| {
            let bounds = to_pixels(item.x, item.y, item.width, item.height, scale);
            let holes = exclusions
                .iter()
                .filter_map(|hole| {
                    let left = hole.x.max(bounds.x);
                    let top = hole.y.max(bounds.y);
                    let right = (hole.x + hole.width).min(bounds.x + bounds.width);
                    let bottom = (hole.y + hole.height).min(bounds.y + bounds.height);
                    (right > left && bottom > top).then_some(PixelRect {
                        x: left - bounds.x,
                        y: top - bounds.y,
                        width: right - left,
                        height: bottom - top,
                    })
                })
                .collect();
            SlotPlan {
                service_id: item.service_id.clone(),
                bounds,
                holes,
                captured: layout.modal_input_capture,
            }
        })
        .collect())
}
