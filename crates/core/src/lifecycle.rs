use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub loaded: bool,
    pub visible: bool,
    pub dirty: bool,
    pub media: bool,
    pub blocked: u64,
    pub last_active: u64,
    pub title: String,
    pub browser_pid: Option<u32>,
    /// Last main-document edge color, sampled by the page-state bridge.
    #[serde(default)]
    pub background: Option<[u8; 3]>,
    #[serde(default)]
    pub last_url: Option<String>,
}

pub fn may_unload(runtime: &Runtime, keep_live: bool) -> bool {
    runtime.loaded && !runtime.visible && !runtime.dirty && !runtime.media && !keep_live
}

/// A soft budget: protected views are never evicted to satisfy a numeric target.
pub fn eviction_candidates<'a>(
    views: impl IntoIterator<Item = (&'a str, &'a Runtime, bool)>,
    budget: usize,
    idle_seconds: u64,
    now: u64,
) -> Vec<String> {
    eviction_candidates_with_timeouts(
        views
            .into_iter()
            .map(|(id, runtime, keep)| (id, runtime, keep, idle_seconds)),
        budget,
        now,
    )
}

pub fn eviction_candidates_with_timeouts<'a>(
    views: impl IntoIterator<Item = (&'a str, &'a Runtime, bool, u64)>,
    budget: usize,
    now: u64,
) -> Vec<String> {
    let views: Vec<_> = views.into_iter().collect();
    let loaded = views.iter().filter(|(_, state, _, _)| state.loaded).count();
    let mut eligible: Vec<_> = views
        .into_iter()
        .filter(|(_, state, keep, _)| may_unload(state, *keep))
        .collect();
    eligible.sort_by_key(|(_, state, _, _)| state.last_active);
    let pressure = loaded.saturating_sub(budget);
    eligible
        .into_iter()
        .enumerate()
        .filter(|(index, (_, state, _, idle_seconds))| {
            *index < pressure || now.saturating_sub(state.last_active) >= *idle_seconds
        })
        .map(|(_, (id, _, _, _))| id.to_string())
        .collect()
}
