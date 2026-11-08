use crate::Result;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub url: String,
    pub workspace_id: String,
    pub profile_name: String,
    pub protection: bool,
    pub keep_live: bool,
    pub color: String,
    #[serde(default)]
    pub idle_minutes: Option<u64>,
    #[serde(default)]
    pub catalog_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewService {
    pub name: String,
    pub url: String,
    pub workspace_id: String,
    #[serde(default)]
    pub keep_live: Option<bool>,
    #[serde(default)]
    pub catalog_id: Option<String>,
}

impl Service {
    pub fn validate_options(&self) -> Result<()> {
        if self
            .idle_minutes
            .is_some_and(|minutes| !(1..=240).contains(&minutes))
        {
            return Err("Choose an idle period from 1–240 minutes.".into());
        }
        if self.catalog_id.as_ref().is_some_and(|id| {
            id.is_empty()
                || id.len() > 80
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        }) {
            return Err("Invalid catalog service identity.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub live_budget: usize,
    pub idle_minutes: u64,
    pub theme: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            live_budget: 5,
            idle_minutes: 15,
            theme: "light".into(),
        }
    }
}

impl Preferences {
    pub fn validate(&self) -> Result<()> {
        if !(1..=30).contains(&self.live_budget) || !(1..=240).contains(&self.idle_minutes) {
            return Err(
                "Choose a live-view budget from 1–30 and an idle period from 1–240 minutes.".into(),
            );
        }
        if !["light", "dark"].contains(&self.theme.as_str()) {
            return Err("Unknown theme.".into());
        }
        Ok(())
    }
}

pub fn service_url(value: &str) -> Result<String> {
    let parsed =
        Url::parse(value.trim()).map_err(|_| "Enter a complete website URL.".to_string())?;
    if !["https", "http"].contains(&parsed.scheme())
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Use an HTTP or HTTPS website without embedded credentials.".into());
    }
    Ok(parsed.to_string())
}

pub fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return Err("Use a name between 1 and 80 characters.".into());
    }
    Ok(value.into())
}
