# Bobola 1.1.0 UI and companion review

## Scope

This review covers the Music and Canvas companions added to the existing Bobola 1.0.1 workspace.

## Changes

- Added compact left-rail launchers for Music and Canvas.
- Added a cobalt-and-cream terminal-inspired music surface with light/dark state, search/play, transport controls, and current-track status.
- Added a QuickDraw infinite canvas surface with local snapshot persistence, undo, clear, theme controls, and upstream watermark.
- Kept companion panels outside native website slots and account profile state.
- Bundled YuTuTui's Windows `ytt.exe` 1.7.9 companion and shipped its MIT license beside the resource.

## Validation

| Check | Result |
| --- | --- |
| `npm run build` | Passed; Svelte check reported 0 errors and 0 warnings |
| `npm test` | Passed: 38 core tests |
| `node scripts/launcher-ui-check.mjs` | Passed: existing navigation, service, theme, catalog, settings, and persistence coverage |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| QuickDraw dependency | `@quickdrawjs/core` 0.3.0, MIT, lockfile pinned |
| YuTuTui companion | `ytt.exe` 1.7.9, MIT license shipped, local controller boundary used |

## Limits

Music playback depends on YuTuTui's `mpv`, `yt-dlp`, and `ffmpeg` helpers and upstream provider availability. The bundled controller started and answered local JSON status requests during packaging validation; live song playback was not exercised. The Bobola panel controls an existing YuTuTui owner; it does not replace YuTuTui's TUI, account, cookies, catalog, or playback engine. QuickDraw boards are local shell data and do not provide multiplayer sync in this integration.
