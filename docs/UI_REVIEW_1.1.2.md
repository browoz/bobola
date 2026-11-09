# Bobola 1.1.2 validation

- Svelte check and production build pass with the native Music component removed.
- Rust formatting and Clippy with warnings denied pass with Music IPC commands removed.
- All 38 core regression tests pass.
- All 16 native smoke checks pass, covering shell IPC, account separation, filtering, persistent profiles, resource reporting, sleep/wake, and the dock Manage dialog.
- Browser regression checks verify no native Music button in the sidebar or Glass Dock, YouTube Music catalog selection and URL, service persistence and opening, and Canvas opening in both navigation modes.
- The browser preview verifies navigation and saved configuration; it does not validate live YouTube authentication or audio playback.

The Windows installer was installed over an existing Bobola installation with its legacy player running. Setup exited successfully, installed version 1.1.2, stopped the matching player, and removed its executable and license. The migration uses native PowerShell so it can inspect the 64-bit player's executable path from the 32-bit installer. No new music player or external playback tools are bundled.
