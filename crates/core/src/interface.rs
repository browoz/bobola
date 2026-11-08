//! Interface preferences. They live in their own `settings` row so the legacy `Preferences`,
//! `Snapshot` and `Service` schemas (all `deny_unknown_fields`) never change.
//! The shape mirrors `docs/INTERFACE_CONTRACT.ts`.
use crate::{Result, model::Preferences};
use serde::{Deserialize, Serialize};

pub const INTERFACE_VERSION: u32 = 1;
/// Upper bound for the persisted JSON row. A valid document is far smaller.
pub const MAX_STORED_BYTES: usize = 4096;
pub const DENSITIES: [u8; 4] = [40, 44, 48, 52];
pub const COMPANION_SIZES: [u8; 3] = [28, 34, 40];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavMode {
    Minimal,
    Autohide,
    Pinned,
    #[serde(rename = "glass-dock")]
    GlassDock,
    #[serde(rename = "orb-launcher")]
    OrbLauncher,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    Lavender,
    Sky,
    Sage,
    Rose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    System,
    Reduced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPrivacy {
    Hidden,
    Sender,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseBehavior {
    Quit,
    Tray,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Companion {
    pub enabled: bool,
    pub size: u8,
    pub motion: Motion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Quiet {
    pub enabled: bool,
    pub hours_enabled: bool,
    /// Local 24-hour `HH:MM`.
    pub from: String,
    /// Local 24-hour `HH:MM`. May be earlier than `from` (an overnight window).
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Notifications {
    pub privacy: NotificationPrivacy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfacePreferences {
    pub version: u32,
    pub nav_mode: NavMode,
    pub density: u8,
    pub theme: ThemeChoice,
    pub accent: Accent,
    pub companion: Companion,
    pub quiet: Quiet,
    pub notifications: Notifications,
    pub close_behavior: CloseBehavior,
}

impl Default for InterfacePreferences {
    fn default() -> Self {
        Self {
            version: INTERFACE_VERSION,
            nav_mode: NavMode::Autohide,
            density: 48,
            theme: ThemeChoice::System,
            accent: Accent::Lavender,
            companion: Companion {
                enabled: true,
                size: 34,
                motion: Motion::System,
            },
            quiet: Quiet {
                enabled: false,
                hours_enabled: false,
                from: "22:00".into(),
                to: "07:00".into(),
            },
            notifications: Notifications {
                privacy: NotificationPrivacy::Hidden,
            },
            close_behavior: CloseBehavior::Quit,
        }
    }
}

/// Strict `HH:MM` (00:00 to 23:59). No seconds, no single-digit hours, no whitespace.
fn valid_clock(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    let digit = |index: usize| bytes[index].is_ascii_digit().then(|| bytes[index] - b'0');
    let (Some(h1), Some(h2), Some(m1), Some(m2)) = (digit(0), digit(1), digit(3), digit(4)) else {
        return false;
    };
    h1 * 10 + h2 <= 23 && m1 * 10 + m2 <= 59
}

impl InterfacePreferences {
    /// Migration defaults for an install that has no interface row yet. The legacy
    /// `Preferences.theme` (`light` or `dark`) is honored; every other option takes its default.
    pub fn from_legacy_theme(theme: &str) -> Result<Self> {
        let theme = match theme {
            "light" => ThemeChoice::Light,
            "dark" => ThemeChoice::Dark,
            _ => return Err("Unknown theme.".into()),
        };
        Ok(Self {
            theme,
            ..Self::default()
        })
    }

    pub fn from_legacy(preferences: &Preferences) -> Result<Self> {
        Self::from_legacy_theme(&preferences.theme)
    }

    /// Structural validation. Everything a stored or imported document must satisfy.
    pub fn validate(&self) -> Result<()> {
        if self.version != INTERFACE_VERSION {
            return Err(format!(
                "Unsupported interface preferences version {}.",
                self.version
            ));
        }
        if !DENSITIES.contains(&self.density) {
            return Err("Choose a density of 40, 44, 48 or 52.".into());
        }
        if !COMPANION_SIZES.contains(&self.companion.size) {
            return Err("Choose a companion size of 28, 34 or 40.".into());
        }
        if !valid_clock(&self.quiet.from) || !valid_clock(&self.quiet.to) {
            return Err("Quiet hours must use 24-hour HH:MM times.".into());
        }
        Ok(())
    }

    /// Validation for anything the user saves or restores. `tray` is reserved until the
    /// backend can keep the app running in the background.
    pub fn validate_for_save(&self) -> Result<()> {
        self.validate()?;
        if self.close_behavior == CloseBehavior::Tray {
            return Err(
                "Closing to the tray is not supported yet. Choose \"quit\" for now.".into(),
            );
        }
        Ok(())
    }

    /// Strict parse of a persisted row. A damaged row is an error, never a silent reset.
    pub fn from_stored(body: &str) -> Result<Self> {
        if body.len() > MAX_STORED_BYTES {
            return Err("Stored interface preferences are too large.".into());
        }
        let parsed: Self = serde_json::from_str(body)
            .map_err(|e| format!("Stored interface preferences are unreadable: {e}"))?;
        parsed
            .validate()
            .map_err(|e| format!("Stored interface preferences are invalid: {e}"))?;
        Ok(parsed)
    }

    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}
