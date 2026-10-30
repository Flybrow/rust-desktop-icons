# Rust Desktop Icons

Open-source Fences 6 alternative for Windows 10/11: customizable desktop rectangles ("fences") that hold icons.

## Primary rule (non-negotiable)
- **No god files**: one responsibility per module. When a file mixes concerns or grows past ~250 lines (data tables like
  `i18n.rs` excepted), split it into a folder module (`foo/mod.rs` + focused files) before adding more.
- **Strict DRY**: any logic written twice is factored (helper fn, shared const, `App`/`win` method). Check existing
  helpers (`win.rs`, `App::fence_of`, `app::change`, `fence::update`) before writing new code.

## Stack
Rust 2024 (1.98), `windows` 0.62 (raw Win32/GDI), serde/serde_json, ureq 3 (rustls), sha2. MSI via WiX 5.0.2.

## Commands (verified)
- Test: `cargo test` — single: `cargo test grid_layout`
- Lint / format: `cargo clippy --all-targets -- -D warnings` / `cargo fmt`
- Build: `cargo build --release`
- MSI (local): needs WiX 5.0.2 (`dotnet tool install wix --version 5.0.2`) + `wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2`:
  `wix build installer/main.wxs -d Version=0.1.0 -arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext -o target/rdi.msi`
- Release: bump `version` in Cargo.toml, write the changelog in English in `.github/release-notes.md` (release body + embedded in the exe, shown once after an update by `whatsnew.rs`), commit, push tag `vX.Y.Z` → `.github/workflows/release.yml` builds and publishes MSI + `.sha256`.

## Layout
- `src/domain/` pure logic, all unit-tested: `model.rs` (Fence/Tab/Look), `config.rs` (Config, recycle placement), `kind.rs` (file types + game
  shortcut detection for rules),
  `order.rs` (per-tab sort + custom order), `anim.rs`, `grid.rs`, `snap.rs`, `icons.rs`, `color.rs`, `zone.rs`.
- `store.rs` persistence (`tab_dir`: portal path or `fences/<tab id>`); `i18n/` (`mod.rs` logic, `table.rs` 25 languages);
  `update/` updater: `release.rs` (pure, tested: GitHub release JSON, versions, notes, checksum, progress), `net.rs` (trusted
  URLs only, streamed download with progress), `install.rs` (msiexec + relaunch chain; debug builds never install), `mod.rs` check/download.
  `tray/updates.rs` drives it: auto (6 h) and manual checks open the themed popup (notes, Install / Later), download progress is
  coalesced (`LATEST` + `QUEUED`, never one message per chunk). Debug `RDI_FAKE_VERSION=0.0.1` pretends to be old (E2E: `update_e2e`).
- `dialog/` themed popup (what's new, update available/progress, notices): `layout.rs` pure + tested, `paint.rs`, `mod.rs` window;
  shares the settings look through `layered/chrome.rs` (background, close button, outline, colors).
  `rules.rs` desktop watcher + auto-sort (files younger than 10 s are retried later); `prefs.rs` global prefs;
  `report.rs` errors: `report::alert` (dialog + `errors.log`), `report::log`, panic hook -> `crash.log`;
  `uninstall.rs` (`--uninstall`, run by the MSI on real uninstall only: moves every `fences/*` folder found on disk back to the desktop, never trusting the config,
  and deletes the app folder only once they are gone).
- `src/keep/` optional "keep files on the desktop" mode (`keep_on_desktop`, off by default): `mod.rs` toggle + migration both ways (`sync`) + `reconcile` (follows renames via file id),
  `migrate.rs` runs the migration (`sync`) on a thread and applies it on the tray thread (`WM_KEEP` -> `finish`),
  `place.rs` drop/paste/rules place files on the Desktop and pin them to a tab, `keeper.rs` hides pinned icons (SetWinEventHook + 3 s timer, Explorer re-shows them on F5/restart) and restores desktop flags.
  Hidden icons are stacked in one free cell at the bottom-right of the primary work area (`corner_slot`) under `cover.rs`, a small window painting the wallpaper (`PaintDesktop`).
  Never park icons off-screen: Explorer pulls every item that is not fully on a monitor back on screen at each drop (visible flash). Hidden icons that get selected are deselected at once.
  `domain/pins.rs` (pure, tested) holds `Pin` + reconcile + free slots + the hidden stack cell; `shell/hide.rs` is the IFolderView2 wrapper. Debug builds only touch the real desktop with `RDI_REAL_DESKTOP=1`.
  End-to-end test: `cargo build && cargo run --example keep_e2e [-- --restart-explorer]` (real Desktop, isolated `LOCALAPPDATA`, test files `*.rdie2e`).
- `src/app/` global state: `mod.rs` App + `with` + startup, `view.rs` per-window state, `res.rs` fonts/glyphs/menu
  entries with icons, `fences.rs` create/rebuild fences; `src/tray/` tray icon/menu (`updates.rs` update scheduling).
- `src/render/` pure pixel ops: `mod.rs` blending + `Canvas`, `pixels.rs` gray/tint/straight alpha, `blur.rs`.
- `src/layered/` shared per-pixel window painting (`Frame`: text mask, compositing, present; `Dib`; `glyph.rs`
  Segoe MDL2 icon glyphs, also rendered to menu bitmaps via `App::entry`/`App::sub`).
- `src/shell/` Win32 shell wrappers: `mod.rs` paths/icons/open, `reg.rs` registry, `link.rs`, `watch.rs` (+ `notify_moved` after every move), `dnd.rs`,
  `system.rs` (system icons: Recycle Bin, This PC, User's Files, Network, Control Panel as `::{CLSID}` items,
  name/icon from the shell, desktop visibility via HideDesktopIcons)
  (folder picker, `drag_out` = OLE drag via SHDoDragDrop with our IDropSource: a drop on the desktop can be cancelled
  for Explorer and handled by the app; the app calls OleInitialize). The keeper never calls Explorer while a mouse button is down.
  E2E helpers shared by the examples live in `examples/support`.
- `src/fence/` fence window: `mod.rs` proc, `layout.rs` geometry, `items.rs` icons, `paint.rs` + `header.rs` rendering,
  `tabs.rs` tabs/portals, `anim.rs` unroll + chameleon fade, `peek.rs` hover, `menu.rs`, `actions.rs`, `title.rs` rename,
  `drop.rs` (from outside / onto a tab header / reorder inside a tab), `select.rs` (click, Ctrl+click, rubber band,
  drag out), `arrange.rs` (per-tab sort and rules submenus), `system.rs` (system icons placement: menu, drag, drop), `snap.rs`.
- System icons: `domain/system.rs` (`Sys`, pure) lives in `Tab.system` (legacy `recycle: true` migrates in `Config::sanitized`);
  `app/desk_icons.rs` hides them on the desktop while placed and restores the original HideDesktopIcons value
  (`Config.desk_icons`) when removed; dropping one on the desktop shows it. E2E: `cargo run --example system_e2e`.
- `src/settings/` settings window (custom drawn): `rows.rs` content/layout, `paint.rs`, `act.rs`, `mod.rs` window.
- `installer/main.wxs` per-user MSI (installs to `%LOCALAPPDATA%\Programs`, owns the HKCU Run value).

## Conventions
- User wants terse code: no comments, compact formatting (`rustfmt.toml` max_width 140).
- Business rules go in `domain.rs` with tests; Win32 code stays thin.
- Global state is `thread_local RefCell` accessed only via `app::with` (try_borrow). Never call Win32 APIs that
  send messages (DestroyWindow, SetWindowPos, TrackPopupMenu, MessageBox, CreateWindow) inside `with`.
- New UI string: add a `T` variant and translate it in all 25 rows of `i18n.rs` (tests check completeness).

## Robustness rules
- Never ignore a user-visible failure: collect errors and call `report::failures`/`report::alert` outside `app::with`.
- `store::save` keeps `config.json.bak`; an unreadable config is renamed `config.json.bad-<ts>` and never overwritten.
- Items keep a system image-list index (`Item.icon: i32`), never an HICON: icons are drawn on demand via
  `shell::draw_icon`. Folder reads + icon lookups run on a thread (`fence/loader.rs`, per-tab generations);
  each tab has its own change message (`WM_CHANGED + tab index`, `WM_LOADED` sits after that range).
- Present layered windows outside `app::with` (UpdateLayeredWindow re-enters the window proc).
- Bar side: `domain/side.rs` (pure, tested) maps between window and canonical grid coordinates for the 4 bar sides
  (`Geo`: bar/body rects, `to_content`/`to_window`, hit zones, `far` rule). `Fence.vertical` puts the bar on the side;
  `layout::orient` picks `View.side` (rolled fences only when folded, open ones at rest): Bottom/Right when there is no
  room below/right. `f.x/f.y` is always the bar top-left; the window grows away from it (`layout::origin`, `crop`).
  Items hug the bar (Bottom/Right mirror the grid). The bar is drawn in its own `Frame`, rotated (`render::rotate_cw`)
  for vertical bars and blitted over the body (`header::paint`). E2E: `cargo run --example unroll_e2e [-- --shots <dir>]`.
- Roll/fade animation never re-renders per frame: `paint::draw(.., open=true)` once into `View.frame`, then
  `Frame::present_top` crops it; ticks use the QPC clock (`win::now_ms`; DWM vblank timing goes stale on an idle desktop, never use it), WM_SIZE skips render
  while animating (`View.sizing` marks self-caused resizes). RAM first: the open frame is cached (canvas only,
  `Frame::compact`) only while an animation can start (`anim::cached`: rolled fence hovered/animating, or chameleon);
  `PREPARE` rebuilds it 150 ms after invalidation. Icons draw straight from the system image list (`IImageList::Draw`, no HICON),
  blur works on u8 buffers, hot paths read the fence via `read(h, ..)` instead of cloning it. `Canvas::rrect` fast-paths straight rows and `blur` only processes rows with text (tests prove equality).

## Gotchas
- Debug builds use a separate single-instance mutex, so they run beside the installed app. For visual checks, run the debug exe
  with `LOCALAPPDATA` pointed at a scratch dir (never touch the user's real data) and `RDI_DUMP=<file>` to dump the canvas.
- Background blur was tried and rejected (v0.3): DWM system backdrops go solid on inactive windows, and the
  undocumented SetWindowCompositionAttribute accent rendered opaque black with GDI content. Keep the layered path.
- Debug `RDI_TOP=1` makes fences topmost and unowned so screenshots can capture them.
- Fences are `UpdateLayeredWindow` windows: no WM_PAINT, call `fence::render`; child controls don't show (rename edit is a popup).
- Fence windows are owned by `Progman` and forced to `HWND_BOTTOM`; Explorer restart → `TaskbarCreated` → `rebuild()`.
- The updater only accepts URLs under this repo's releases; the MSI asset must have a matching `<name>.msi.sha256`.
- MSI `UpgradeCode` and component GUIDs must never change.
- The MSI closes the running app first (`util:CloseApplication` → WM_CLOSE; Restart Manager → WM_QUERYENDSESSION/WM_ENDSESSION,
  both handled by the tray window, which saves and quits). Relaunch after an update: the updater runs msiexec through
  `cmd /c ... & start exe --relaunch` (`update::chain`, tested) and the MSI `Launch` action also passes `--relaunch`;
  with that flag `app::instance::claim` waits up to 15 s for the old instance's mutex, then exits quietly if one still runs.
  Start and close reasons go to `errors.log`. E2E: `cargo run --example relaunch_e2e`.
- The desktop right-click verb (`HKCU\Software\Classes\DesktopBackground\Shell\RustDesktopIcons`) is written by the
  release app at startup (localized, runs `exe --new`), removed by the MSI on uninstall. On Windows 11 it only shows under
  "Show more options" (the modern menu needs a packaged IExplorerCommand).
- `store::move_into` refuses to move a folder into itself and only falls back to copy+delete across volumes (never deletes a
  complete copy). Use `fence::view(h, ..)` for per-window state, `Config::tab` for fence/tab lookups.
- Desktop files are always moved, never copied: public-desktop items go through `shell::transfer` (UAC), only files
  from other folders become shortcuts. Debug builds never touch the real desktop (verb, system icon visibility).
- MSI desktop shortcut: created on first install (checkbox, default on). On upgrades it is only recreated if it still
  exists on the desktop (`SHORTCUTEXISTS` FileSearch), so a shortcut the user deleted never comes back.
