//! Pure popup policy: target validation, flood limiting, activity aggregation and titles.
use crate::{Result, lifecycle::Runtime, model::service_url};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use url::Url;

/// Page activity reported by one popup. Kept apart from the main view's `Runtime` flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupState {
    pub dirty: bool,
    pub media: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PopupActivity {
    pub open: usize,
    pub dirty: bool,
    pub media: bool,
}

pub fn summarize<'a>(states: impl IntoIterator<Item = &'a PopupState>) -> PopupActivity {
    states
        .into_iter()
        .fold(PopupActivity::default(), |mut total, state| {
            total.open += 1;
            total.dirty |= state.dirty;
            total.media |= state.media;
            total
        })
}

/// Returns the main-view runtime with popup activity folded in, for unload decisions only.
/// An open popup counts as visible so automatic unloading refuses the account.
pub fn with_popups(runtime: &Runtime, activity: PopupActivity) -> Runtime {
    let mut merged = runtime.clone();
    merged.visible |= activity.open > 0;
    merged.dirty |= activity.dirty;
    merged.media |= activity.media;
    merged
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupTarget {
    /// `window.open()` with no URL yet; the opener navigates it afterwards.
    Blank,
    Web(String),
}

/// Accepts about:blank and HTTP(S) without credentials. Everything else is refused.
pub fn popup_target(value: &str) -> Result<PopupTarget> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(PopupTarget::Blank);
    }
    if let Ok(parsed) = Url::parse(trimmed)
        && parsed.scheme() == "about"
    {
        return if parsed.path() == "blank"
            && parsed.query().is_none()
            && parsed.fragment().is_none()
        {
            Ok(PopupTarget::Blank)
        } else {
            Err("Only about:blank is allowed for about: popups.".into())
        };
    }
    service_url(trimmed)
        .map(PopupTarget::Web)
        .map_err(|_| "Popups may only open about:blank or HTTP(S) websites.".to_string())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupLimits {
    pub per_account: usize,
    pub total: usize,
    pub burst: usize,
    pub window_seconds: u64,
}

impl Default for PopupLimits {
    fn default() -> Self {
        Self {
            per_account: 4,
            total: 12,
            burst: 6,
            window_seconds: 10,
        }
    }
}

/// Counts open popups and recent open attempts per account. `now` is injected for tests.
#[derive(Debug, Default)]
pub struct PopupGate {
    limits: PopupLimits,
    open: BTreeMap<String, usize>,
    recent: BTreeMap<String, VecDeque<u64>>,
}

impl PopupGate {
    pub fn new(limits: PopupLimits) -> Self {
        Self {
            limits,
            ..Self::default()
        }
    }

    /// Reserves a slot. Every successful call must be paired with one `release`.
    pub fn admit(&mut self, account: &str, now: u64) -> Result<()> {
        let window = self.limits.window_seconds;
        let account_open = self.open_count(account);
        let total_open: usize = self.open.values().sum();
        let recent = self.recent.entry(account.to_string()).or_default();
        while recent
            .front()
            .is_some_and(|at| now.saturating_sub(*at) >= window)
        {
            recent.pop_front();
        }
        if recent.len() >= self.limits.burst {
            return Err("This service is opening popups too quickly. Try again shortly.".into());
        }
        if account_open >= self.limits.per_account {
            return Err("This service already has the maximum number of popups open.".into());
        }
        if total_open >= self.limits.total {
            return Err("Too many popups are open. Close one and try again.".into());
        }
        recent.push_back(now);
        *self.open.entry(account.to_string()).or_default() += 1;
        Ok(())
    }

    pub fn release(&mut self, account: &str) {
        if let Some(count) = self.open.get_mut(account) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.open.remove(account);
            }
        }
    }

    pub fn open_count(&self, account: &str) -> usize {
        self.open.get(account).copied().unwrap_or(0)
    }
}

/// Origin shown in a popup title. Derived from the browser-reported URL, never from page content.
pub fn origin_label(source: &str) -> String {
    match Url::parse(source) {
        Ok(url) if url.scheme() == "about" && url.path() == "blank" => "about:blank".into(),
        Ok(url) if ["http", "https"].contains(&url.scheme()) && url.host_str().is_some() => {
            url.origin().ascii_serialization()
        }
        _ => "unverified page".into(),
    }
}

pub fn popup_title(account_name: &str, source: &str) -> String {
    let name: String = account_name
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    format!("{name} - {}", origin_label(source))
}
