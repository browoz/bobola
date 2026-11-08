# Implementation and current scope

Bobola is a Windows-first desktop workspace built with Rust, Tauri 2, native WebView2 views, and a Svelte 5 shell.

## Implemented

- Service and workspace creation, account switching, two-column service views, and searchable settings.
- Native account profiles in one shared WebView2 service environment. Native checks verify separate cookies/local storage and retained profiles after restart.
- Direct SQLite configuration storage, validated backups, and configuration restore.
- Brave `adblock-rust` matching integrated into native request events, plus main-document cosmetic filtering.
- Per-service protection control, a small original starter list, explicit EasyList/EasyPrivacy downloads, and list rollback.
- A soft live-view budget and idle unloading. Keep-live defaults protect notification services; media and changed-form guards are conservative and best effort.
- Metadata from 441 pinned Ferdium recipe entries. This is catalog coverage, not 441 ported integrations. See [recipe support](../RECIPE_SUPPORT.json).

## Native boundaries and local data

Remote service views do not receive Tauri's application command bridge. Their native message handler accepts bounded page-state information for cosmetic matching and conservative lifecycle guards. It does not expose filesystem, process, or configuration commands.

Website sign-ins remain inside their browser profiles. Configuration export does not copy cookies or passwords. Removing a service removes its configuration while retaining its browser profile; secure profile deletion remains a separate feature.

The browser preview is an explicit shell preview. Its local storage is separate from native SQLite, and it does not emulate successful live protection, native views, or authentication.

## Filtering scope

Cosmetic filtering currently targets the main document with bounded class/id discovery. Procedural rules, scriptlet resources, redirect resources, precise initiator context in every frame/worker, and universal video-ad removal are not implemented. Request initiator fallback uses the current top-level URL when no referrer is available. A filter update requires an explicit action.

The integrated engine is Brave's Rust adblock engine, not an AdGuard integration. Broad site compatibility and real-world savings require additional measurement.

## Lifecycle and compatibility

Unloading cannot guarantee real-time notifications for arbitrary services. Popular messaging and email accounts default to keep-live; custom services default to sleeping when eligible. Form-change detection is a conservative heuristic, not a universal guarantee of draft preservation.

Full recipe behavior, unread adapters, authentication edge cases, native notifications, call/screen-share compatibility, proxy controls, custom scripts/styles, Todos, sync, and migration from Ferdium remain tracked requirements. Account-scoped popups are implemented with bounded creation and native isolation checks; individual provider authentication flows still need manual verification.

## Validation and provenance

The 1.0.1 review covers 38 core tests, 16 native smoke checks, popup/overlay fixtures, and browser UI regression checks. See [the release review](UI_REVIEW_1.0.1.md) for evidence and limitations. No total-resource saving percentage against Ferdium has been established.

The target baseline is Ferdium 7.2.3 at `a44bc7d85dea106aaf7f7f6e2c9820597d38ebc6`, with recipes at `bc708f6aad02fc43168e976d1d65678eaf96f21b`. Functional parity also requires manual review of behaviors that static analysis cannot see. See [parity tracking](PARITY.md).
