# Third-party notices

The generated `docs/DEPENDENCY_NOTICES.txt` lists the resolved Windows Rust and production npm dependencies, license texts found in their packages, and pinned source archive URLs. It is also included beside the Windows installer.

| Component | Use | License / source |
| --- | --- | --- |
| adblock-rust | Unmodified Rust request/cosmetic matching library | MPL-2.0, https://github.com/brave/adblock-rust |
| Tauri and WRY | Desktop shell and browser host | Apache-2.0 OR MIT, https://github.com/tauri-apps/tauri and https://github.com/tauri-apps/wry |
| Svelte | Local shell | MIT, https://github.com/sveltejs/svelte |
| Lucide | Interface icons | ISC and retained upstream icon notices, https://github.com/lucide-icons/lucide |
| SQLite / rusqlite | Local configuration storage | SQLite public domain; rusqlite MIT, https://sqlite.org and https://github.com/rusqlite/rusqlite |
| Ferdium recipe metadata | Names, URLs and configuration flags from pinned recipe manifests | Individual recipe licenses recorded in `src/lib/catalog.json`; source revision in `CATALOG_PROVENANCE.json` |
| Ferdium service artwork | 30 locally bundled SVG logos | Pinned sources and hashes in `SERVICE_ARTWORK_PROVENANCE.json`; MIT text and attribution in `public/service-icons/`; brand trademarks remain with their owners |
| EasyList and EasyPrivacy | Optional lists downloaded by the user at runtime | Retain list metadata and review the source's applicable GPL/Creative Commons notices, https://easylist.to/ |
| YuTuTui companion | Optional Windows `ytt.exe` music controller | MIT, https://github.com/Ochichan/Yututui; release license text is shipped as `src-tauri/resources/YUTUTUI_LICENSE.md` |
| QuickDraw | Local infinite canvas engine (`@quickdrawjs/core`) | MIT, https://github.com/quickdrawjs/quickdraw |

Keep MPL-covered library source available under its applicable terms when distributing a binary. This application does not relabel third-party components as Apache-only code. Catalog metadata import is distinct from porting or redistributing executable recipe scripts. The small bundled starter filter list is original project code.

Ferdium was studied from publicly available Apache-2.0 application source and a copied installed package. Source study is documented; this project does not claim a clean-room process. Product and service names identify compatibility targets, and no affiliation is implied.

OmpDeck's theme structure was inspected as a design reference at revision `071ef923e20529b9b59fb4a69d54ad499266b123`. No OmpDeck runtime, fonts or source files are bundled.
