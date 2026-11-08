use rusqlite::Connection;
use std::path::{Path, PathBuf};
use workspace_core::{
    interface::{
        Accent, CloseBehavior, Companion, InterfacePreferences, Motion, NavMode,
        NotificationPrivacy, Notifications, Quiet, ThemeChoice,
    },
    model::{NewService, Preferences, Service},
    store::{Backup, BackupV2, MAX_BACKUP_BYTES, Store},
};

fn memory() -> Store {
    Store::open(Path::new(":memory:")).unwrap()
}
fn file_store() -> (tempfile::TempDir, PathBuf, Store) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.db");
    let store = Store::open(&path).unwrap();
    (directory, path, store)
}
fn input(name: &str) -> NewService {
    NewService {
        name: name.into(),
        url: "https://example.com".into(),
        workspace_id: "personal".into(),
        keep_live: None,
        catalog_id: None,
    }
}
fn custom() -> InterfacePreferences {
    InterfacePreferences {
        version: 1,
        nav_mode: NavMode::Pinned,
        density: 52,
        theme: ThemeChoice::Dark,
        accent: Accent::Rose,
        companion: Companion {
            enabled: false,
            size: 40,
            motion: Motion::Reduced,
        },
        quiet: Quiet {
            enabled: true,
            hours_enabled: true,
            from: "23:30".into(),
            to: "06:15".into(),
        },
        notifications: Notifications {
            privacy: NotificationPrivacy::Preview,
        },
        close_behavior: CloseBehavior::Quit,
    }
}
fn legacy(theme: &str) -> Preferences {
    Preferences {
        live_budget: 7,
        idle_minutes: 20,
        theme: theme.into(),
    }
}
/// Everything a restore may touch, as one comparable string.
fn state(db: &Store) -> String {
    serde_json::to_string(&(db.snapshot().unwrap(), db.interface_preferences().unwrap())).unwrap()
}

#[test]
fn navigation_modes_persist_after_reopen_and_backup_restore() {
    for (name, mode) in [
        ("minimal", NavMode::Minimal),
        ("autohide", NavMode::Autohide),
        ("pinned", NavMode::Pinned),
        ("glass-dock", NavMode::GlassDock),
        ("orb-launcher", NavMode::OrbLauncher),
    ] {
        let (_directory, path, mut db) = file_store();
        let service = db.add_service(input("Keep account")).unwrap();
        let mut payload = serde_json::to_value(custom()).unwrap();
        payload["navMode"] = name.into();
        payload["density"] = 40.into();
        // Deserialize the same spelling sent by the frontend command.
        let preferences: InterfacePreferences = serde_json::from_value(payload).unwrap();
        assert_eq!(preferences.nav_mode, mode);
        db.set_interface_preferences(&preferences).unwrap();
        drop(db);

        let db = Store::open(&path).unwrap();
        assert_eq!(db.interface_preferences().unwrap(), preferences);
        assert_eq!(db.snapshot().unwrap().services, vec![service.clone()]);
        let backup = serde_json::to_value(db.backup().unwrap()).unwrap();
        assert_eq!(backup["interfacePreferences"]["navMode"], name);
        let mut target = memory();
        target
            .restore_backup(Backup::parse(&backup.to_string()).unwrap())
            .unwrap();
        assert_eq!(target.interface_preferences().unwrap(), preferences);
        assert_eq!(target.snapshot().unwrap().services, vec![service]);
    }
}

#[test]
fn legacy_v1_backup_roundtrips_and_resets_the_sidecar_from_its_theme() {
    let mut source = memory();
    let service = source.add_service(input("Mail")).unwrap();
    source.set_preferences(&legacy("dark")).unwrap();
    let v1_text = serde_json::to_string(&source.snapshot().unwrap()).unwrap();

    // The target already has customised, unrelated interface options.
    let mut target = memory();
    target.add_service(input("Old")).unwrap();
    target.set_interface_preferences(&custom()).unwrap();

    match Backup::parse(&v1_text).unwrap() {
        Backup::Legacy(snapshot) => target.restore(snapshot).unwrap(),
        Backup::V2(_) => panic!("a bare snapshot must parse as the legacy format"),
    }

    let restored = target.snapshot().unwrap();
    assert_eq!(restored.services.len(), 1);
    assert_eq!(restored.services[0], service);
    assert_eq!(restored.preferences.theme, "dark");
    assert_eq!(restored.preferences.live_budget, 7);
    // No stale per-user options: only the imported legacy theme carries over.
    assert_eq!(
        target.interface_preferences().unwrap(),
        InterfacePreferences::from_legacy_theme("dark").unwrap()
    );
}

#[test]
fn v2_backup_roundtrips_the_sidecar_and_contains_nothing_else() {
    let mut source = memory();
    source.add_service(input("Chat")).unwrap();
    source.set_preferences(&legacy("light")).unwrap();
    source.set_interface_preferences(&custom()).unwrap();
    let text = serde_json::to_string_pretty(&source.backup().unwrap()).unwrap();

    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["interfacePreferences", "snapshot", "version"]);
    assert_eq!(value["version"], 2);
    assert_eq!(value["snapshot"]["version"], 1);

    let mut target = memory();
    target.add_service(input("Replaced")).unwrap();
    target
        .restore_backup(Backup::parse(&text).unwrap())
        .unwrap();
    assert_eq!(target.interface_preferences().unwrap(), custom());
    assert_eq!(state(&target), state(&source));
}

#[test]
fn missing_sidecar_migrates_the_legacy_theme_until_the_user_saves() {
    let mut db = memory();
    assert_eq!(
        db.interface_preferences().unwrap(),
        InterfacePreferences::default()
    );
    db.set_preferences(&legacy("dark")).unwrap();
    assert_eq!(db.interface_preferences().unwrap().theme, ThemeChoice::Dark);

    let mut explicit = custom();
    explicit.theme = ThemeChoice::System;
    db.set_interface_preferences(&explicit).unwrap();
    db.set_preferences(&legacy("light")).unwrap();
    assert_eq!(db.interface_preferences().unwrap(), explicit);
}

#[test]
fn invalid_clocks_density_size_and_version_are_rejected_without_changing_saved_data() {
    let mut db = memory();
    db.set_interface_preferences(&custom()).unwrap();
    let before = state(&db);

    for clock in [
        "24:00", "7:00", "07:60", "07:0", "0700", "07:00 ", "ab:cd", "07:00:00", "", "+7:00",
    ] {
        let mut bad = custom();
        bad.quiet.from = clock.into();
        assert!(
            db.set_interface_preferences(&bad).is_err(),
            "from {clock:?}"
        );
        let mut bad = custom();
        bad.quiet.to = clock.into();
        assert!(db.set_interface_preferences(&bad).is_err(), "to {clock:?}");
    }
    for clock in ["00:00", "23:59", "12:30"] {
        let mut good = custom();
        good.quiet.from = clock.into();
        good.quiet.to = clock.into();
        db.set_interface_preferences(&good).unwrap();
    }
    db.set_interface_preferences(&custom()).unwrap();

    let mut bad = custom();
    bad.density = 50;
    assert!(db.set_interface_preferences(&bad).is_err());
    let mut bad = custom();
    bad.companion.size = 30;
    assert!(db.set_interface_preferences(&bad).is_err());
    for version in [0, 2, 99] {
        let mut bad = custom();
        bad.version = version;
        assert!(
            db.set_interface_preferences(&bad).is_err(),
            "version {version}"
        );
    }
    assert_eq!(state(&db), before);
}

#[test]
fn tray_close_behavior_is_an_honest_unsupported_error() {
    let mut db = memory();
    db.set_interface_preferences(&custom()).unwrap();
    let before = state(&db);
    let mut tray = custom();
    tray.close_behavior = CloseBehavior::Tray;
    let error = db.set_interface_preferences(&tray).unwrap_err();
    assert!(error.contains("not supported"), "{error}");
    assert_eq!(state(&db), before);

    // A backup carrying tray is refused too, and leaves everything untouched.
    let mut backup = db.backup().unwrap();
    backup.interface_preferences.close_behavior = CloseBehavior::Tray;
    assert!(db.restore_backup(Backup::V2(backup)).is_err());
    assert_eq!(state(&db), before);
}

#[test]
fn unknown_enums_missing_fields_and_extra_fields_fail_to_parse() {
    let good = serde_json::to_value(custom()).unwrap();
    assert!(serde_json::from_value::<InterfacePreferences>(good.clone()).is_ok());

    for (path, value) in [
        ("navMode", "sidebar"),
        ("theme", "sepia"),
        ("accent", "teal"),
        ("closeBehavior", "minimize"),
    ] {
        let mut bad = good.clone();
        bad[path] = value.into();
        assert!(
            serde_json::from_value::<InterfacePreferences>(bad).is_err(),
            "{path}"
        );
    }
    let mut bad = good.clone();
    bad["companion"]["motion"] = "fast".into();
    assert!(serde_json::from_value::<InterfacePreferences>(bad).is_err());
    let mut bad = good.clone();
    bad["notifications"]["privacy"] = "everything".into();
    assert!(serde_json::from_value::<InterfacePreferences>(bad).is_err());

    let parents: [Option<&str>; 4] = [
        None,
        Some("quiet"),
        Some("companion"),
        Some("notifications"),
    ];
    for parent in parents {
        let mut bad = good.clone();
        match parent {
            Some(key) => bad[key]["unexpected"] = true.into(),
            None => bad["unexpected"] = true.into(),
        }
        assert!(
            serde_json::from_value::<InterfacePreferences>(bad).is_err(),
            "{parent:?}"
        );
    }
    for field in ["version", "density", "quiet", "closeBehavior"] {
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<InterfacePreferences>(bad).is_err(),
            "{field}"
        );
    }
}

#[test]
fn a_damaged_stored_row_is_an_error_and_is_never_reset() {
    let (_directory, path, mut db) = file_store();
    db.set_interface_preferences(&custom()).unwrap();
    let valid_body = serde_json::to_string(&custom()).unwrap();
    let damaged = [
        "{not json".to_string(),
        valid_body.replace("\"density\":52", "\"density\":51"),
        valid_body.replace("23:30", "25:30"),
        valid_body.replace("\"version\":1", "\"version\":2"),
        valid_body.replace(
            "\"closeBehavior\":\"quit\"}",
            "\"closeBehavior\":\"quit\",\"extra\":1}",
        ),
        format!("{valid_body}{}", " ".repeat(5000)),
    ];
    for body in damaged {
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE settings SET body=?1 WHERE key='interface_preferences'",
                [&body],
            )
            .unwrap();
        assert!(db.interface_preferences().is_err(), "{body:.60}");
        assert!(db.backup().is_err(), "export must not hide damage");
    }
    // Saving a valid document is the explicit way out.
    db.set_interface_preferences(&custom()).unwrap();
    assert_eq!(db.interface_preferences().unwrap(), custom());
}

#[test]
fn rejected_restores_keep_prior_services_and_preferences() {
    let mut db = memory();
    db.add_service(input("Keep me")).unwrap();
    db.set_preferences(&legacy("dark")).unwrap();
    db.set_interface_preferences(&custom()).unwrap();
    let before = state(&db);

    let mut donor = memory();
    donor.add_service(input("Incoming")).unwrap();
    let good = donor.backup().unwrap();

    let mut bad_density = good.clone();
    bad_density.interface_preferences.density = 45;
    let mut bad_clock = good.clone();
    bad_clock.interface_preferences.quiet.to = "24:01".into();
    let mut bad_envelope = good.clone();
    bad_envelope.version = 3;
    let mut bad_snapshot_version = good.clone();
    bad_snapshot_version.snapshot.version = 2;
    let mut bad_profile = good.clone();
    bad_profile.snapshot.services[0].profile_name = "../../outside".into();
    let mut bad_workspace = good.clone();
    bad_workspace.snapshot.services[0].workspace_id = "missing".into();
    let mut bad_legacy_theme = good.clone();
    bad_legacy_theme.snapshot.preferences.theme = "sepia".into();
    let mut bad_budget = good.clone();
    bad_budget.snapshot.preferences.live_budget = 0;

    for (label, backup) in [
        ("density", bad_density),
        ("clock", bad_clock),
        ("envelope version", bad_envelope),
        ("snapshot version", bad_snapshot_version),
        ("profile", bad_profile),
        ("workspace", bad_workspace),
        ("legacy theme", bad_legacy_theme),
        ("budget", bad_budget),
    ] {
        assert!(db.restore_backup(Backup::V2(backup)).is_err(), "{label}");
        assert_eq!(state(&db), before, "{label} changed stored data");
    }
    let mut legacy_bad = donor.snapshot().unwrap();
    legacy_bad.preferences.theme = "sepia".into();
    assert!(db.restore(legacy_bad).is_err());
    assert_eq!(state(&db), before);
}

#[test]
fn a_failure_inside_the_transaction_rolls_back_every_table() {
    let (_directory, path, mut db) = file_store();
    db.add_service(input("Keep me")).unwrap();
    db.set_preferences(&legacy("dark")).unwrap();
    db.set_interface_preferences(&custom()).unwrap();
    let before = state(&db);

    let mut donor = memory();
    donor.add_service(input("Incoming")).unwrap();
    donor.set_preferences(&legacy("light")).unwrap();
    let mut incoming = donor.backup().unwrap();
    incoming.interface_preferences = InterfacePreferences::default();

    // The sidecar write is the last statement. Services, workspaces and legacy preferences
    // have already been replaced when it fails, so only a real rollback can restore them.
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fail_sidecar BEFORE INSERT ON settings
             WHEN NEW.key='interface_preferences'
             BEGIN SELECT RAISE(ABORT,'sidecar write refused'); END;",
        )
        .unwrap();
    let error = db.restore_backup(Backup::V2(incoming.clone())).unwrap_err();
    assert!(error.contains("sidecar write refused"), "{error}");
    assert_eq!(state(&db), before);

    Connection::open(&path)
        .unwrap()
        .execute_batch("DROP TRIGGER fail_sidecar")
        .unwrap();
    db.restore_backup(Backup::V2(incoming)).unwrap();
    assert_eq!(db.snapshot().unwrap().services[0].name, "Incoming");
    assert_eq!(
        db.interface_preferences().unwrap(),
        InterfacePreferences::default()
    );
}

#[test]
fn backup_size_and_collection_limits_are_enforced() {
    let mut db = memory();
    db.set_interface_preferences(&custom()).unwrap();
    let before = state(&db);

    let oversized = " ".repeat(MAX_BACKUP_BYTES + 1);
    assert!(Backup::parse(&oversized).unwrap_err().contains("4 MB"));

    let base = db.backup().unwrap();
    let service = |index: usize| Service {
        id: format!("s{index}"),
        name: format!("Service {index}"),
        url: "https://example.com/".into(),
        workspace_id: "personal".into(),
        profile_name: format!("account_s{index}"),
        protection: true,
        keep_live: true,
        color: "blue".into(),
        idle_minutes: None,
        catalog_id: None,
    };
    let workspace = |index: usize| workspace_core::model::Workspace {
        id: format!("w{index}"),
        name: format!("Workspace {index}"),
    };

    let mut at_limit: BackupV2 = base.clone();
    at_limit.snapshot.services = (0..1000).map(service).collect();
    at_limit.snapshot.workspaces.extend((0..99).map(workspace));
    assert_eq!(at_limit.snapshot.workspaces.len(), 100);
    db.restore_backup(Backup::V2(at_limit.clone())).unwrap();
    assert_eq!(db.snapshot().unwrap().services.len(), 1000);
    let at_limit_state = state(&db);

    let mut too_many_services = at_limit.clone();
    too_many_services.snapshot.services.push(service(1000));
    assert!(db.restore_backup(Backup::V2(too_many_services)).is_err());
    let mut too_many_workspaces = at_limit;
    too_many_workspaces.snapshot.workspaces.push(workspace(99));
    assert!(db.restore_backup(Backup::V2(too_many_workspaces)).is_err());
    assert_eq!(state(&db), at_limit_state);
    assert_ne!(at_limit_state, before);
}

#[test]
fn backup_parsing_rejects_unknown_fields_and_ambiguous_versions() {
    let mut db = memory();
    db.add_service(input("Mail")).unwrap();
    let v2 = serde_json::to_value(db.backup().unwrap()).unwrap();
    let v1 = serde_json::to_value(db.snapshot().unwrap()).unwrap();
    assert!(Backup::parse(&v2.to_string()).is_ok());
    assert!(Backup::parse(&v1.to_string()).is_ok());

    let with = |value: &serde_json::Value, edit: &dyn Fn(&mut serde_json::Value)| {
        let mut copy = value.clone();
        edit(&mut copy);
        copy.to_string()
    };
    // Unknown fields at every level.
    assert!(Backup::parse(&with(&v2, &|v| v["extra"] = 1.into())).is_err());
    assert!(Backup::parse(&with(&v2, &|v| v["snapshot"]["extra"] = 1.into())).is_err());
    assert!(Backup::parse(&with(&v2, &|v| v["interfacePreferences"]["extra"] = 1.into())).is_err());
    assert!(
        Backup::parse(&with(&v2, &|v| v["snapshot"]["services"][0]["extra"] = 1.into())).is_err()
    );
    assert!(Backup::parse(&with(&v1, &|v| v["extra"] = 1.into())).is_err());
    // A v1 document may not smuggle in the sidecar.
    let sidecar = v2["interfacePreferences"].clone();
    assert!(Backup::parse(&with(&v1, &|v| v["interfacePreferences"] = sidecar.clone())).is_err());
    // A v2 envelope needs its sidecar.
    assert!(
        Backup::parse(&with(&v2, &|v| {
            v.as_object_mut().unwrap().remove("interfacePreferences");
        }))
        .is_err()
    );
    // Versions: only 1 (bare snapshot) and 2 (envelope).
    for version in [
        serde_json::json!(0),
        3.into(),
        "2".into(),
        serde_json::Value::Null,
    ] {
        assert!(Backup::parse(&with(&v2, &|v| v["version"] = version.clone())).is_err());
        assert!(Backup::parse(&with(&v1, &|v| v["version"] = version.clone())).is_err());
    }
    assert!(
        Backup::parse(&with(&v2, &|v| {
            v.as_object_mut().unwrap().remove("version");
        }))
        .is_err()
    );
    assert!(Backup::parse("[]").is_err());
    assert!(Backup::parse("not json").is_err());
}

#[test]
fn backups_hold_only_workspace_data() {
    let mut db = memory();
    db.add_service(input("Mail")).unwrap();
    let text = serde_json::to_string(&db.backup().unwrap())
        .unwrap()
        .to_lowercase();
    for forbidden in ["cookie", "session", "token", "password", "secret", "key\":"] {
        assert!(!text.contains(forbidden), "backup mentions {forbidden}");
    }
}
