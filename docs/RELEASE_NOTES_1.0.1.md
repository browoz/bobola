# Bobola 1.0.1

- Fixed Manage, Reload and Close view from the service dock context menu. The callback now receives the selected service before the menu state is cleared.
- Fixed a queued dialog-close event cancelling a newly opening Manage dialog.
- Refined compact light/dark surfaces, sidebar row density, focus and pressed states; removed dock backdrop blur and heavy shadows.
- Applied the launch-film color treatment to the existing Bobola mark and regenerated native icons. Added Bobola to the Windows title bar and expanded sidebar.
- Preserved the narrow dock, optional sidebar, service sessions and existing conservative sleep policy.
- Restored the exact pinned Telegram artwork bytes and added Git attributes to preserve artwork hashes.

Validation and measurement scope are documented in `docs/UI_REVIEW_1.0.1.md`. The Windows installer is unsigned and uses the WebView2 download bootstrapper if needed. Signed-in third-party authentication, calls and notifications require service-specific manual verification; fixture tests cannot establish universal compatibility.
