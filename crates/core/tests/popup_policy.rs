use workspace_core::{
    lifecycle::{Runtime, eviction_candidates, may_unload},
    popup::*,
};

#[test]
fn allows_blank_and_http_targets_only() {
    assert_eq!(popup_target("").unwrap(), PopupTarget::Blank);
    assert_eq!(popup_target("about:blank").unwrap(), PopupTarget::Blank);
    assert!(matches!(
        popup_target("https://accounts.example.com/login?x=1").unwrap(),
        PopupTarget::Web(_)
    ));
    assert!(matches!(
        popup_target("http://example.com").unwrap(),
        PopupTarget::Web(_)
    ));
    for refused in [
        "javascript:alert(1)",
        "file:///C:/Windows/win.ini",
        "data:text/html,hi",
        "ms-settings:network",
        "custom-protocol://x",
        "about:srcdoc",
        "about:blank?x=1",
        &("https://user:".to_owned() + "pass@example.com/"),
        "blob:https://example.com/id",
    ] {
        assert!(popup_target(refused).is_err(), "{refused} must be refused");
    }
}

#[test]
fn gate_limits_open_popups_per_account_and_total() {
    let mut gate = PopupGate::new(PopupLimits {
        per_account: 2,
        total: 3,
        burst: 100,
        window_seconds: 10,
    });
    gate.admit("a", 0).unwrap();
    gate.admit("a", 0).unwrap();
    assert!(gate.admit("a", 0).is_err());
    gate.admit("b", 0).unwrap();
    assert!(gate.admit("c", 0).is_err(), "total cap");
    gate.release("a");
    gate.admit("c", 0).unwrap();
    assert_eq!(gate.open_count("a"), 1);
}

#[test]
fn gate_throttles_bursts_and_recovers() {
    let mut gate = PopupGate::new(PopupLimits {
        per_account: 100,
        total: 100,
        burst: 3,
        window_seconds: 10,
    });
    for _ in 0..3 {
        gate.admit("a", 5).unwrap();
        gate.release("a");
    }
    assert!(gate.admit("a", 6).is_err());
    assert!(gate.admit("b", 6).is_ok(), "other accounts are independent");
    gate.admit("a", 15).unwrap();
}

#[test]
fn failed_reservation_release_restores_capacity() {
    let mut gate = PopupGate::new(PopupLimits {
        per_account: 1,
        total: 1,
        burst: 100,
        window_seconds: 10,
    });
    gate.admit("a", 0).unwrap();
    gate.release("a");
    gate.release("a");
    assert_eq!(gate.open_count("a"), 0);
    gate.admit("a", 1).unwrap();
}

#[test]
fn popup_activity_blocks_unloading_without_touching_main_flags() {
    let main = Runtime {
        loaded: true,
        ..Runtime::default()
    };
    assert!(may_unload(&main, false));
    let activity = summarize(&[PopupState {
        dirty: false,
        media: false,
    }]);
    let merged = with_popups(&main, activity);
    assert!(!may_unload(&merged, false));
    assert!(
        !main.dirty && !main.media && !main.visible,
        "input is not modified"
    );
    let busy = with_popups(
        &main,
        summarize(&[PopupState {
            dirty: true,
            media: true,
        }]),
    );
    assert!(busy.dirty && busy.media);
    assert!(!with_popups(&main, summarize(&[])).visible);
}

#[test]
fn eviction_skips_accounts_with_popups() {
    let idle = Runtime {
        loaded: true,
        last_active: 0,
        ..Runtime::default()
    };
    let with_popup = with_popups(&idle, summarize(&[PopupState::default()]));
    let candidates = eviction_candidates(
        [("plain", &idle, false), ("popup", &with_popup, false)],
        1,
        60,
        10_000,
    );
    assert_eq!(candidates, vec!["plain".to_string()]);
}

#[test]
fn titles_use_verified_origin_and_clean_names() {
    assert_eq!(
        popup_title("Mail", "https://login.example.com:8443/a?b=c"),
        "Mail - https://login.example.com:8443"
    );
    assert_eq!(popup_title("Mail", "about:blank"), "Mail - about:blank");
    assert_eq!(
        popup_title("Mail", "javascript:1"),
        "Mail - unverified page"
    );
    assert_eq!(popup_title("Ma\u{0}il\n", ""), "Mail - unverified page");
    assert!(popup_title(&"x".repeat(500), "https://a.test/").len() < 100);
}
