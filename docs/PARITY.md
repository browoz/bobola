# Functional parity tracker

Baseline: Ferdium 7.2.3 and pinned recipes. Status as of 2026-11-04. The complete user-facing feature set remains the target.

| Area | Current result | Remaining work |
| --- | --- | --- |
| Separate accounts | Cookie/local-storage separation and restart persistence verified in native fixtures | Provider sign-ins and unusual session behaviors |
| Efficient profiles | Two account views verified to share one browser process | Matched workloads at larger scale; proxy/profile combinations |
| Service views | Native Windows views and basic loading work | Full navigation, recovery, popup and permission behavior |
| Workspaces/split views | Creation, filtering and two-pane behavior checked in the shell | Complete upstream ordering, startup, layout and settings coverage |
| Keyboard switching | Shell Ctrl+K exercised, including global search across workspaces | Native shortcut handling while a remote service owns focus |
| Catalog | Metadata for 441 entries imported | Every site-specific adapter remains unported/unverified in `RECIPE_SUPPORT.json` |
| Unread counts | No generic recipe badge implementation yet | Reviewed per-service adapters and notification integration |
| Embedded blocking | Native request blocking, main-document cosmetic filtering and per-service disabling verified | Precise context in every frame/worker, procedural filters, redirect/scriptlet resources and broader compatibility |
| Filter maintenance | Downloads, validation, bounded current/previous storage and rollback implemented; matching/rollback tests pass | Scheduling, richer diagnostics and update-failure UI scenarios |
| Resource controls | Soft-budget decisions tested; native dirty-state reporting checked | Call/transfer guard coverage and real workload measurements |
| Persistence | SQLite, validated configuration backups and atomic rejection tested | Complete migrations, corruption recovery and Ferdium imports |
| App appearance | Light/dark shell settings and searchable controls | Full service-specific dark mode/customization parity |
| Notifications | Not implemented/verified as a complete native feature | Permission flow, privacy, actions, sound, taskbar and DND behavior |
| Calls/screen sharing | Not verified with providers | Popup/permission implementation and owner-controlled call tests |
| Proxies/certificates | Not implemented | Complete service and environment controls |
| User scripts/styles | Not implemented | Reviewed extension contract preserving user-facing behavior |
| Todos | Not implemented | Companion panel and provider integration |
| Cloud sync | Not implemented | Account/server adapters, offline changes and conflict handling |
| Profile deletion | Configuration removal retains browser profiles, explicitly disclosed | Safe per-profile data removal while preserving other accounts |
| Distribution | Windows x64 per-user NSIS installer, launch notes, license notices and checksums | Code signing certificate, updater and full installer distribution automation |

Passing native fixtures does not prove compatibility with every service. The static ledger's 99 candidate expectations require human review and additional acceptance cases; they do not substitute for this functional tracker or real-service verification.
