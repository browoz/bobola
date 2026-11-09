# Bobola 1.1.2 — Website music playback

- Removed the native Music panel and its navigation buttons, Rust commands, status polling, and YuTuTui download step.
- The Windows package no longer bundles YuTuTui or requires its external playback tools.
- Use **Add service → YouTube Music** to add `https://music.youtube.com/` as a normal website service.
- Canvas remains available from the sidebar and Glass Dock.
- Windows setup removes the retired bundled player from older Bobola installations, stopping only matching player processes inside the installation directory. Personal YuTuTui data is retained.
