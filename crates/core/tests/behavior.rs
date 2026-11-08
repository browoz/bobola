use std::path::Path;
use workspace_core::{
    lifecycle::{Runtime, eviction_candidates},
    model::{NewService, service_url},
    protection::Protection,
    store::Store,
};

fn store() -> Store {
    Store::open(Path::new(":memory:")).unwrap()
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

#[test]
fn accounts_get_distinct_persistent_profiles() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.db");
    let mut db = Store::open(&path).unwrap();
    let a = db.add_service(input("First")).unwrap();
    let b = db.add_service(input("Second")).unwrap();
    assert_ne!(a.profile_name, b.profile_name);
    drop(db);
    let restored = Store::open(&path).unwrap().snapshot().unwrap();
    assert_eq!(restored.services[0].profile_name, a.profile_name);
    assert_eq!(restored.services.len(), 2);
}

#[test]
fn credentials_and_native_urls_are_rejected() {
    for url in [
        "file:///C:/secret",
        "javascript:alert(1)",
        &("https://user:".to_owned() + "password@example.com"),
        "data:text/html,hello",
    ] {
        assert!(service_url(url).is_err());
    }
    assert!(service_url("http://localhost:8080").is_ok());
}

#[test]
fn backup_restore_is_atomic_and_validates_profile_identity() {
    let mut db = store();
    db.add_service(input("Keep me")).unwrap();
    let mut backup = db.snapshot().unwrap();
    backup.services[0].profile_name = "../../outside".into();
    assert!(db.restore(backup).is_err());
    assert_eq!(db.snapshot().unwrap().services[0].name, "Keep me");
}

#[test]
fn invalid_workspace_does_not_create_a_service() {
    let mut db = store();
    let mut entry = input("Invalid");
    entry.workspace_id = "missing".into();
    assert!(db.add_service(entry).is_err());
    assert!(db.snapshot().unwrap().services.is_empty());
}

#[test]
fn matching_supports_exceptions_and_per_service_disable() {
    let engine = Protection::compile("||ads.example^\n@@||ads.example/allowed.js$script").unwrap();
    assert!(
        engine
            .check(
                "https://ads.example/ad.js",
                "https://site.example",
                "script",
                true
            )
            .blocked
    );
    assert!(
        !engine
            .check(
                "https://ads.example/allowed.js",
                "https://site.example",
                "script",
                true
            )
            .blocked
    );
    assert!(
        !engine
            .check(
                "https://ads.example/ad.js",
                "https://site.example",
                "script",
                false
            )
            .blocked
    );
}

#[test]
fn third_party_context_is_used() {
    let engine = Protection::compile("||assets.example^$third-party").unwrap();
    assert!(
        engine
            .check(
                "https://assets.example/script.js",
                "https://other.example",
                "script",
                true
            )
            .blocked
    );
    assert!(
        !engine
            .check(
                "https://assets.example/script.js",
                "https://assets.example/page",
                "script",
                true
            )
            .blocked
    );
}

#[test]
fn cosmetic_rules_and_generic_exceptions_are_respected() {
    let engine =
        Protection::compile("example.com##.sponsored\n##.advert\nexample.com#@#.advert").unwrap();
    let css = engine.cosmetic_css("https://example.com", &["advert".into()], &[]);
    assert!(css.contains(".sponsored"));
    assert!(!css.contains(".advert{"));
}

#[test]
fn bad_filter_update_preserves_current_revision_and_rollback_restores_previous() {
    let mut db = store();
    let a = db.apply_filters("||first.example^").unwrap();
    assert!(db.apply_filters("<html>server error</html>").is_err());
    assert_eq!(
        Protection::compile(&db.filter_text().unwrap())
            .unwrap()
            .revision,
        a.revision
    );
    let second = db.apply_filters("||second.example^").unwrap();
    db.apply_filters("||third.example^").unwrap();
    assert_eq!(db.rollback_filters().unwrap().revision, second.revision);
}

#[test]
fn budget_protects_active_media_dirty_and_kept_live_services() {
    let normal = Runtime {
        loaded: true,
        last_active: 1,
        ..Default::default()
    };
    let media = Runtime {
        media: true,
        ..normal.clone()
    };
    let dirty = Runtime {
        dirty: true,
        ..normal.clone()
    };
    let visible = Runtime {
        visible: true,
        ..normal.clone()
    };
    let result = eviction_candidates(
        [
            ("idle", &normal, false),
            ("call", &media, false),
            ("draft", &dirty, false),
            ("active", &visible, false),
            ("notifications", &normal, true),
        ],
        1,
        60,
        1000,
    );
    assert_eq!(result, vec!["idle"]);
}

#[test]
fn new_sleep_policy_preserves_legacy_accounts_and_roundtrips_options() {
    let mut db = store();
    let mut entry = db.add_service(input("Occasional tool")).unwrap();
    assert!(!entry.keep_live);
    entry.idle_minutes = Some(3);
    entry.catalog_id = Some("github".into());
    db.save_service(&entry).unwrap();
    assert_eq!(db.snapshot().unwrap().services[0], entry);
    let mut legacy = serde_json::to_value(&entry).unwrap();
    legacy.as_object_mut().unwrap().remove("idleMinutes");
    legacy.as_object_mut().unwrap().remove("catalogId");
    legacy["keepLive"] = true.into();
    let restored: workspace_core::model::Service = serde_json::from_value(legacy).unwrap();
    assert!(restored.keep_live);
    assert_eq!(restored.idle_minutes, None);
    let mut live_input = input("Messaging");
    live_input.keep_live = Some(true);
    assert!(db.add_service(live_input).unwrap().keep_live);
    let mut invalid = db.snapshot().unwrap();
    invalid.services[0].idle_minutes = Some(0);
    assert!(db.restore(invalid).is_err());
    assert_eq!(db.snapshot().unwrap().services[0], entry);
    for timeout in [0, 241] {
        let mut rejected = entry.clone();
        rejected.idle_minutes = Some(timeout);
        assert!(db.save_service(&rejected).is_err());
    }
    for catalog_id in ["", "../escape", "service name"] {
        let mut rejected = entry.clone();
        rejected.catalog_id = Some(catalog_id.into());
        assert!(db.save_service(&rejected).is_err());
    }
}

#[test]
fn per_account_timeouts_and_budget_still_protect_live_activity() {
    use workspace_core::lifecycle::eviction_candidates_with_timeouts;
    let idle = Runtime {
        loaded: true,
        last_active: 100,
        ..Default::default()
    };
    let active = Runtime {
        visible: true,
        ..idle.clone()
    };
    let entries = || {
        [
            ("short", &idle, false, 60),
            ("long", &idle, false, 600),
            ("live", &idle, true, 60),
            ("visible", &active, false, 60),
        ]
    };
    assert_eq!(
        eviction_candidates_with_timeouts(entries(), 5, 200),
        vec!["short"]
    );
    assert_eq!(
        eviction_candidates_with_timeouts(entries(), 2, 200),
        vec!["short", "long"]
    );
}
