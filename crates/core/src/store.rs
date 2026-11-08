use crate::{
    Result,
    interface::InterfacePreferences,
    model::{self, NewService, Preferences, Service, Workspace},
    protection::{Protection, STARTER_FILTERS},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

/// Largest accepted backup document.
pub const MAX_BACKUP_BYTES: usize = 4 * 1024 * 1024;
const INTERFACE_KEY: &str = "interface_preferences";

pub struct Store {
    db: Connection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub services: Vec<Service>,
    pub workspaces: Vec<Workspace>,
    pub preferences: Preferences,
}

/// Backup format 2. Wraps the unchanged version 1 `Snapshot` and adds the interface sidecar.
/// Holds no cookies, sessions or keys: only what `Snapshot` and `InterfacePreferences` carry.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupV2 {
    pub version: u32,
    pub snapshot: Snapshot,
    pub interface_preferences: InterfacePreferences,
}

/// A parsed backup of either supported format.
#[derive(Clone, Debug)]
pub enum Backup {
    /// Format 1: a bare `Snapshot`. Importing it resets the interface sidecar to migration defaults.
    Legacy(Snapshot),
    V2(BackupV2),
}

impl Backup {
    /// Parses and size-checks backup text. The format is chosen by the top-level `version`
    /// and each format is then parsed strictly, so unknown fields are rejected.
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() > MAX_BACKUP_BYTES {
            return Err("Backup exceeds 4 MB.".into());
        }
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("Invalid backup: {e}"))?;
        match value.get("version").and_then(serde_json::Value::as_u64) {
            Some(1) => serde_json::from_value(value)
                .map(Backup::Legacy)
                .map_err(|e| format!("Invalid backup: {e}")),
            Some(2) => serde_json::from_value(value)
                .map(Backup::V2)
                .map_err(|e| format!("Invalid backup: {e}")),
            _ => Err("Unsupported backup version.".into()),
        }
    }
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        db.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS workspaces(id TEXT PRIMARY KEY, name TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS services(id TEXT PRIMARY KEY, body TEXT NOT NULL, workspace TEXT NOT NULL REFERENCES workspaces(id));
          CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, body TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS filters(id INTEGER PRIMARY KEY AUTOINCREMENT, body TEXT NOT NULL);
          INSERT OR IGNORE INTO workspaces VALUES('personal','Personal');").map_err(|e| e.to_string())?;
        Ok(Self { db })
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut services = self
            .db
            .prepare("SELECT body FROM services ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        let services: Vec<Service> = services
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .map(|value| {
                value
                    .map_err(|e| e.to_string())
                    .and_then(|body| serde_json::from_str(&body).map_err(|e| e.to_string()))
            })
            .collect::<Result<_>>()?;
        let mut workspaces = self
            .db
            .prepare("SELECT id,name FROM workspaces ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        let workspaces = workspaces
            .query_map([], |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let preferences = self.stored_legacy_preferences()?.unwrap_or_default();
        Ok(Snapshot {
            version: 1,
            services,
            workspaces,
            preferences,
        })
    }

    fn setting(&self, key: &str) -> Result<Option<String>> {
        self.db
            .query_row("SELECT body FROM settings WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())
    }

    fn stored_legacy_preferences(&self) -> Result<Option<Preferences>> {
        self.setting("preferences")?
            .map(|body| serde_json::from_str(&body).map_err(|e| e.to_string()))
            .transpose()
    }

    /// Interface preferences from their own row. Without a row, the legacy theme is migrated
    /// (a fresh install with no legacy row gets plain defaults). A damaged row is an error.
    pub fn interface_preferences(&self) -> Result<InterfacePreferences> {
        if let Some(body) = self.setting(INTERFACE_KEY)? {
            return InterfacePreferences::from_stored(&body);
        }
        match self.stored_legacy_preferences()? {
            Some(legacy) => InterfacePreferences::from_legacy(&legacy),
            None => Ok(InterfacePreferences::default()),
        }
    }

    /// Saves from the UI. Rejects `closeBehavior: "tray"` until the backend supports it.
    pub fn set_interface_preferences(&mut self, preferences: &InterfacePreferences) -> Result<()> {
        preferences.validate_for_save()?;
        let body = preferences.to_json()?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO settings(key,body) VALUES(?1,?2)",
                params![INTERFACE_KEY, body],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Version 2 backup of the current state.
    pub fn backup(&self) -> Result<BackupV2> {
        Ok(BackupV2 {
            version: 2,
            snapshot: self.snapshot()?,
            interface_preferences: self.interface_preferences()?,
        })
    }

    pub fn add_service(&mut self, input: NewService) -> Result<Service> {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let service = Service {
            profile_name: format!("account_{id}"),
            id,
            name: model::name(&input.name)?,
            url: model::service_url(&input.url)?,
            workspace_id: input.workspace_id,
            protection: true,
            keep_live: input.keep_live.unwrap_or(false),
            color: "blue".into(),
            idle_minutes: None,
            catalog_id: input.catalog_id,
        };
        self.save_service(&service)?;
        Ok(service)
    }

    pub fn save_service(&mut self, service: &Service) -> Result<()> {
        service.validate_options()?;
        model::name(&service.name)?;
        model::service_url(&service.url)?;
        if !valid_id(&service.id) || service.profile_name != format!("account_{}", service.id) {
            return Err("Invalid account profile identity.".into());
        }
        let body = serde_json::to_string(service).map_err(|e| e.to_string())?;
        self.db.execute("INSERT INTO services(id,body,workspace) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET body=excluded.body,workspace=excluded.workspace", params![service.id,body,service.workspace_id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_service(&mut self, id: &str) -> Result<()> {
        self.db
            .execute("DELETE FROM services WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn add_workspace(&mut self, name: &str) -> Result<Workspace> {
        let workspace = Workspace {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name: model::name(name)?,
        };
        self.db
            .execute(
                "INSERT INTO workspaces VALUES(?1,?2)",
                params![workspace.id, workspace.name],
            )
            .map_err(|e| e.to_string())?;
        Ok(workspace)
    }
    pub fn set_preferences(&mut self, preferences: &Preferences) -> Result<()> {
        preferences.validate()?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO settings VALUES('preferences',?1)",
                [serde_json::to_string(preferences).map_err(|e| e.to_string())?],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn filter_text(&self) -> Result<String> {
        Ok(self
            .db
            .query_row(
                "SELECT body FROM filters ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| STARTER_FILTERS.into()))
    }
    pub fn apply_filters(&mut self, text: &str) -> Result<Protection> {
        let next = Protection::compile(text)?;
        let transaction = self.db.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute("INSERT INTO filters(body) VALUES(?1)", [text])
            .map_err(|e| e.to_string())?;
        transaction.execute("DELETE FROM filters WHERE id NOT IN (SELECT id FROM filters ORDER BY id DESC LIMIT 2)", []).map_err(|e|e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(next)
    }
    pub fn rollback_filters(&mut self) -> Result<Protection> {
        let transaction = self.db.transaction().map_err(|e| e.to_string())?;
        let current: Option<i64> = transaction
            .query_row(
                "SELECT id FROM filters ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let previous: String = transaction
            .query_row(
                "SELECT body FROM filters ORDER BY id DESC LIMIT 1 OFFSET 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| STARTER_FILTERS.into());
        let engine = Protection::compile(&previous)?;
        if let Some(id) = current {
            transaction
                .execute("DELETE FROM filters WHERE id=?1", [id])
                .map_err(|e| e.to_string())?;
        }
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(engine)
    }
    /// Restores a version 1 snapshot. The interface sidecar is reset to migration defaults
    /// derived from the imported legacy theme, so no stale per-user options survive.
    pub fn restore(&mut self, snapshot: Snapshot) -> Result<()> {
        self.restore_backup(Backup::Legacy(snapshot))
    }

    /// Restores a backup of either format. Everything is validated before the single
    /// transaction that replaces services, workspaces, legacy preferences and the sidecar.
    pub fn restore_backup(&mut self, backup: Backup) -> Result<()> {
        let (snapshot, interface) = match backup {
            Backup::Legacy(snapshot) => {
                let interface = InterfacePreferences::from_legacy(&snapshot.preferences)?;
                (snapshot, interface)
            }
            Backup::V2(backup) => {
                if backup.version != 2 {
                    return Err("Unsupported or oversized backup.".into());
                }
                backup.interface_preferences.validate_for_save()?;
                (backup.snapshot, backup.interface_preferences)
            }
        };
        let interface_body = interface.to_json()?;
        if snapshot.version != 1
            || snapshot.services.len() > 1000
            || snapshot.workspaces.len() > 100
        {
            return Err("Unsupported or oversized backup.".into());
        }
        snapshot.preferences.validate()?;
        let workspaces: HashSet<_> = snapshot
            .workspaces
            .iter()
            .map(|workspace| &workspace.id)
            .collect();
        if workspaces.len() != snapshot.workspaces.len() || workspaces.is_empty() {
            return Err("Invalid workspace list.".into());
        }
        let mut ids = HashSet::new();
        for workspace in &snapshot.workspaces {
            model::name(&workspace.name)?;
            if !valid_id(&workspace.id) {
                return Err("Invalid workspace identity.".into());
            }
        }
        for service in &snapshot.services {
            service.validate_options()?;
            model::name(&service.name)?;
            model::service_url(&service.url)?;
            if !ids.insert(&service.id)
                || !valid_id(&service.id)
                || service.profile_name != format!("account_{}", service.id)
                || !workspaces.contains(&service.workspace_id)
            {
                return Err("Invalid account identity or workspace in backup.".into());
            }
        }
        let transaction = self.db.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM services", [])
            .map_err(|e| e.to_string())?;
        transaction
            .execute("DELETE FROM workspaces", [])
            .map_err(|e| e.to_string())?;
        for workspace in snapshot.workspaces {
            transaction
                .execute(
                    "INSERT INTO workspaces VALUES(?1,?2)",
                    params![workspace.id, workspace.name],
                )
                .map_err(|e| e.to_string())?;
        }
        for service in snapshot.services {
            transaction
                .execute(
                    "INSERT INTO services VALUES(?1,?2,?3)",
                    params![
                        service.id,
                        serde_json::to_string(&service).map_err(|e| e.to_string())?,
                        service.workspace_id
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        transaction
            .execute(
                "INSERT OR REPLACE INTO settings VALUES('preferences',?1)",
                [serde_json::to_string(&snapshot.preferences).map_err(|e| e.to_string())?],
            )
            .map_err(|e| e.to_string())?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO settings(key,body) VALUES(?1,?2)",
                params![INTERFACE_KEY, interface_body],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
