# Contributing

## Primary rule
- **No god files**: one responsibility per module. Split a file into a folder module as soon as it mixes concerns
  or grows past ~250 lines (pure data tables such as `src/i18n/table.rs` excepted).
- **Strict DRY**: never write the same logic twice; factor it into a helper and reuse existing ones first.

## Before a pull request
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` must pass (CI enforces them).
- Business rules go in `src/domain/` with unit tests; Win32 code stays thin.
- New UI text: add a `T` variant and translate it in all 25 languages of `src/i18n/table.rs`.

## Build from source

```powershell
cargo test
cargo build --release
dotnet tool install --global wix --version 5.0.2
wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2
wix build installer/main.wxs -d Version=X.Y.Z -arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext -o rust-desktop-icons.msi
```

End-to-end checks on a real desktop (isolated data, test files only): `cargo build` then
`cargo run --example keep_e2e`, `cargo run --example unroll_e2e`, `cargo run --example system_e2e`, `cargo run --example relaunch_e2e`,
`cargo run --example update_e2e`, `cargo run --example layout_e2e [-- --desktop]`, `cargo run --example peek_e2e`,
`menu_e2e`, `actions_e2e` (every fence and item menu command), `settings_e2e` (every settings page), `tray_e2e`, `features_e2e`,
`rules_e2e`, `browse_e2e` (folders and folder shortcuts opened inside fences), `display_e2e`, `filter_e2e`, `look_e2e` (corners, list view,
search field, theme and privacy mode as drawn), `views_e2e` (details, tiles and content views, column headers), `perf_e2e` (animation smoothness, run with the release build), `undo_e2e`, `recover_e2e`,
`load_e2e` (41 fences, 3,800 files), `soak_e2e` (resources stay flat), `vdesk_e2e` (creates and closes a temporary virtual desktop)
and `uninstall_e2e`. They move the mouse and press keys:
don't touch the PC while they run. CI runs the ones that need no input on every push.

Pushing a `vX.Y.Z` tag builds and publishes the MSI with [GitHub Actions](.github/workflows/release.yml).

## Architecture

| Module | Role |
| --- | --- |
| `domain/` | Pure, unit-tested logic: model, config, look inheritance, theme, privacy, layout grid, title bar side, sorting, rules, animation, pins, system icons, snapping, screen setups and layouts, shortcuts, search ranking |
| `render/`, `layered/` | Pixel compositing (alpha, rounded rectangles, blur) and layered-window painting |
| `fence/` | Fence window: layout, painting, tabs, drag & drop, selection, menus, roll animation |
| `keep/` | "Keep files on the desktop" mode: migration, placement, hiding desktop icons |
| `hook/` | Small DLL loaded into Explorer that leaves pinned icons out of the desktop view (built and embedded by `build.rs`) |
| `shell/` | Windows Shell wrappers: icons, shortcuts, folder watch, drag & drop, system icons, desktop view |
| `app/`, `tray/`, `settings/`, `dialog/` | Global state and screen changes, tray icon, layouts menu and update scheduling, settings window, themed popups (what's new, updates) |
| `visibility/`, `search/`, `prompt.rs` | Quick hide (desktop double-click), peek shortcut and dim layer, search window, text prompt (layout names) |
| `store/`, `rules/`, `update/` | Persistence and safe moves, desktop auto-sort and age rules, release check and silent install |
| `undo.rs`, `backup.rs`, `uninstall.rs` | Undo (Ctrl+Z), export / import, moving fenced files back to the desktop on uninstall |


## Updates and security

- Every 6 hours (first check 30 s after start) the app asks the GitHub API for the latest release and shows what's new;
  **Install** downloads it with a progress bar, **Later** skips it until the next start.
- A newer `.msi` and its `.sha256` are downloaded **only** from this repository's release URLs over HTTPS; the hash
  must match before `msiexec /qn` runs. The installer then restarts the app.
- Disable it from the tray menu (**Automatic updates**).
- Only documented Windows APIs are used. Dependencies are scanned with `cargo audit` on every push and before each release.
