# Bobola 1.1.0 — Music and Canvas companions

Bobola 1.1.0 adds two optional native companions to the left navigation while preserving the existing web-service workspace and account profiles.

## New

- **Music panel:** terminal-inspired light/dark controls powered by the optional YuTuTui `ytt` companion. Search and play a track, pause/resume, skip, adjust volume, and see current playback status.
- **QuickDraw canvas:** local infinite canvas for freehand drawing, shapes, arrows, sticky notes, diagrams, undo/redo, PNG export, and light/dark themes.
- **Bundled YuTuTui Windows controller:** the Windows installer includes the upstream `ytt.exe` 1.7.9 binary and its MIT license text. Playback still requires `mpv`, `yt-dlp`, and `ffmpeg` as described by YuTuTui. The controller itself was verified to start and answer local JSON status requests in this build; a live song was not played during packaging validation.
- **Clear boundaries:** Music and Canvas are separate from WebView2 account profiles. The canvas board stays in local Bobola shell storage; YuTuTui uses its own per-user data and local control interface.

## Compatibility

- The normal Bobola web-service navigation, protection, profile isolation, sleep policy, and memory controls remain unchanged.
- Music controls start the YuTuTui daemon when needed and communicate through its local `ytt -r` interface. No network listener or account cookie is added by Bobola.
- QuickDraw is MIT licensed and runs locally in the shell with no service account or network requirement.

## Validation

The release passes Svelte checking and production build, the existing core suite, launcher UI regression checks, Rust formatting, Rust clippy, and native compilation. Native service smoke checks remain part of the packaging command.

The Windows installer is unsigned and uses the WebView2 download bootstrapper if WebView2 is missing. YuTuTui playback follows YouTube/provider availability and upstream runtime requirements.
