# Rust Desktop Icons

Open-source Fences 6 alternative for Windows 10/11: customizable desktop rectangles ("fences") that hold icons.

## Primary rule (non-negotiable)
- **No god files**: one responsibility per module. When a file mixes concerns or grows past ~250 lines (data tables like
  `i18n.rs` excepted), split it into a folder module (`foo/mod.rs` + focused files) before adding more.
- **Strict DRY**: any logic written twice is factored (helper fn, shared const, `App`/`win` method). Check existing
  helpers (`win.rs`: `register_with`, `progman`, `keep_bottom`, `work_area_at`, `wheel`, `typed`; `App::fence_of`,
  `Config::fence`, `app::edit`/`app::change` (mutate + save + report), `fence::update`, `layered::popup` (themed popup
  windows: open (sized for the target monitor's DPI)/show/hit/resize/paint/follow_dpi), `tray::timers` (every tray timer
  id + `set`), `win::edit_popup`/`take_text` (popup text box, used by fence rename and `prompt::ask`)) before writing new code.

## Stack
Rust 2024 (1.98), `windows` 0.62 (raw Win32/GDI), serde/serde_json, ureq 3 (rustls), sha2. MSI via WiX 5.0.2. The CRT is linked statically (`.cargo/config.toml`, `+crt-static`):
the exe must never import `VCRUNTIME140.dll` (a clean Windows does not have it; the MSI ships only the exe).

## Commands (verified)
- Test: `cargo test` — single: `cargo test grid_layout`
- Lint / format: `cargo clippy --all-targets -- -D warnings` / `cargo fmt`
- Build: `cargo build --release`
- MSI (local): needs WiX 5.0.2 (`dotnet tool install wix --version 5.0.2`) + `wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2`:
  `wix build installer/main.wxs -d Version=0.1.0 -arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext -o target/rdi.msi`
- Release: bump `version` in Cargo.toml, write the changelog in English in `.github/release-notes.md` (release body + embedded in the exe, shown once after an update by `whatsnew.rs`), commit, push tag `vX.Y.Z` → `.github/workflows/release.yml` builds and publishes MSI + `.sha256`.

## Layout
- `src/domain/` pure logic, all unit-tested: `model.rs` (Fence/Tab), `look.rs` (Look: colors, icons, names, title bar, wallpaper, thumbnails),
  `history.rs` (undo steps: config before + file ops, nesting, cap 20, `rebase` so an undo is never recorded), `config.rs` (Config, recycle placement), `kind.rs` (file types + game
  shortcut detection for rules; `Kind::Other` = "Everything else", the inbox rule `Config::rule_for` falls back to),
  `order.rs` (per-tab sort + custom order), `anim.rs`, `grid.rs` (`Flow`: item positions with `Align` left/center/right,
  last row included; build it with `fence::layout::flow`, which mirrors the alignment for right-side bars), `icons.rs`, `color.rs`, `zone.rs`,
  `snap.rs` (screen edges, other fences' edges and equal spacing -> `Guide` lines, icon-grid sizes `grid_len`),
  `screens.rs` (monitor setups: `Config::adapt` keeps / scales / restores a per-setup `Layout`, setting `screen_adapt`),
  `layouts.rs` (`Layout`, named `Snapshot`s from the tray menu, `Config::restore` scales them to the current screens,
  `Config::duplicate` + `beside`: copy of a fence next to it (else the nearest free spot, never over another fence), new ids, no files/pins/system icons/group),
  `compat.rs` (`known`/`or_default` deserializers: unknown enum values written by a newer version are dropped or
  defaulted instead of rejecting the whole config; use them on every config enum field),
  `stack.rs` (pure plan for "keep files on the desktop": hidden stack cell, icons to hide/show and where, `row` for drops),
  `inherit.rs` (look inheritance: `Fence.custom` lists the `Field`s changed on that fence; `Config::inherit` copies every
  other field from `Config.look`, run on load (legacy configs: fields that differ from the default become custom) and after
  any Settings change; per-fence edits must go through `Fence::set(field, ..)`; `reset_look` clears overrides),
  `hotkey.rs` (`Hotkey` mods/vk + label, reserved system combos rejected, `Slot` peek/search/hide all/new fence/settings:
  all configurable, never equal; `Config::key`/`set_key`/`sanitize_keys`),
  `matching.rs` (`glob` name patterns, `NameRule` (checked before kind rules), per-tab `Expire` age rule, `Config::targets`/`place_name`),
  `groups.rs` (`Fence.group` + `mates`/`group_with`/`ungroup`; virtual desktops: registry GUID parsing, `Fence::shown_on`),
  `backup.rs` (`Config::import` keeps this PC's fields and prefs, `rewind` (undo) restores prefs too; both list orphan tabs to empty and system icons to restore),
  `search.rs` (accent-folding ranking, result place label, double-click test).
  `recover.rs` (`Config::orphans`/`recover`: non-empty `fences/<id>` folders no tab uses come back at startup in a
  "Recovered files" fence, `app::recover` + `store::filled_dirs`). Test-only: `upgrade.rs` loads real configs of every
  released shape (`fixtures/v*.json`, add one when the config format changes), `props.rs` proptest invariants (grid,
  bar sides, screen scaling, `beside`, snapping, globs, search, random/damaged JSON configs: unique fence/tab ids,
  `sanitized` idempotent).
- `store.rs` persistence (`Config::parse` accepts a UTF-8 BOM; `tab_dir`: portal path or `fences/<tab id>`); `i18n/` (`mod.rs` logic, `keys.rs` the `T` enum, `table.rs` 25 languages);
  `backup.rs` export/import of the config JSON (file dialogs in `shell/pick.rs`; import confirms; `adopt` empties orphan tab folders to the desktop and rebuilds, shared with undo);
  `undo.rs` (`step(|| action)` around every user action entry point: fence/tray menus, drop, keys, moves, settings acts, prompts, rules;
  `store::move_into`, `link_into` and item renames note file ops; `undo()` moves files back in reverse (`revert`, unit-tested: never overwrites, skips what is gone, reports locked files) then `adopt(rewind)`; Ctrl+Z via `undo::shortcut`);
  `update/` updater: `release.rs` (pure, tested: GitHub release JSON, versions, notes, checksum, progress), `net.rs` (trusted
  URLs only, streamed download with progress), `install.rs` (msiexec + relaunch chain; debug builds never install), `mod.rs` check/download.
  `tray/updates.rs` drives it: auto (6 h) and manual checks open the themed popup (notes, Install / Later), download progress is
  coalesced (`LATEST` + `QUEUED`, never one message per chunk). Debug `RDI_FAKE_VERSION=0.0.1` pretends to be old (E2E: `update_e2e`).
- `dialog/` themed popup (what's new, update available/progress, notices): `layout.rs` pure + tested, `paint.rs`, `mod.rs` window;
  notes are shown through `update::summary` (section titles + "Title — first sentence" per bullet, full notes behind "Read everything");
  the body height is a whole number of lines with a scrollbar, and the dialog and the settings window re-fit their screen on
  WM_EXITSIZEMOVE/WM_DISPLAYCHANGE (a window moved to another screen of the same DPI gets no WM_DPICHANGED);
  shares the settings look through `layered/chrome.rs` (background, close button, outline, colors).
  `rules/` desktop watcher + auto-sort (`mod.rs`; files younger than 10 s are retried later; `put` is the shared mover),
  `rules/age.rs` age rules (tray timer `AGE`: 1 min after start, then hourly; debug `RDI_AGE_MS` shortens it); new desktop files Explorer drops under a
  fence are moved to a free visible cell (`Desk::uncover` with `fence::footprints`, never while a button is down); `prefs.rs` global prefs;
  `report.rs` errors: `report::alert` (dialog + `errors.log`), `report::log`, panic hook -> `crash.log`;
  `uninstall.rs` (`--uninstall`, run by the MSI on real uninstall only: moves every `fences/*` folder found on disk back to the desktop, never trusting the config,
  and deletes the app folder only once they are gone).
- `src/keep/` optional "keep files on the desktop" mode (`keep_on_desktop`, off by default): `mod.rs` toggle + migration both ways (`sync`) + `reconcile` (follows renames via file id),
  `migrate.rs` runs the migration (`sync`) on a thread and applies it on the tray thread (`WM_KEEP` -> `finish`),
  `place.rs` drop/paste/rules place files on the Desktop and pin them to a tab, `keeper.rs` hides pinned icons (SetWinEventHook + 3 s timer, Explorer re-shows them on F5/restart) and restores desktop flags.
  Hidden icons are stacked in one free cell at the bottom-right of the primary work area (`corner_slot`) under `cover.rs`, a small window painting the wallpaper (`PaintDesktop`).
  When the stack moves to another cell, the cover spans both cells for 250 ms (Explorer moves the icons after the cover).
  Never park icons off-screen: Explorer pulls every item that is not fully on a monitor back on screen at each drop (visible flash). Hidden icons that get selected are deselected at once.
  `domain/pins.rs` (pure, tested) holds `Pin` + reconcile + free slots + the hidden stack cell; `shell/hide.rs` is the IFolderView2 wrapper. Debug builds only touch the real desktop with `RDI_REAL_DESKTOP=1`.
  End-to-end test: `cargo build && cargo run --example keep_e2e [-- --restart-explorer]` (real Desktop, isolated `LOCALAPPDATA`, test files `*.rdie2e`).
- `src/app/` global state: `mod.rs` App + `with` + startup, `view.rs` per-window state, `res.rs` fonts/glyphs/menu
  entries with icons, `fences.rs` create/rebuild fences, `screens.rs` monitor list (adapted at startup and 1.5 s after the
  tray gets `WM_DISPLAYCHANGE`, then `rebuild`); `src/tray/` tray icon/menu (`updates.rs` update scheduling).
- `src/render/` pure pixel ops: `mod.rs` blending + `Canvas`, `pixels.rs` gray/tint/straight alpha, `blur.rs`.
- `src/layered/` shared per-pixel window painting (`Frame`: text mask, compositing, present; `Dib`; `glyph.rs`
  Segoe MDL2 icon glyphs, also rendered to menu bitmaps via `App::entry`/`App::sub`).
- `src/shell/` Win32 shell wrappers: `mod.rs` paths/icons/open, `reg.rs` registry (`reg_bytes`), `thumb.rs` (IShellItemImageFactory thumbnails, made in the loader thread for `kind::thumbable` files at the scaled icon size,
  kept in `Item.thumb`, blitted with `render::blit_fit`), `vdesk.rs` virtual desktops
  (current + ids from HKCU VirtualDesktops, Win10 SessionInfo fallback, names), `pick.rs` folder/JSON file dialogs,
  `menu.rs` (item context menu: `Pick::Shell`/`Own`, our ids from `OWN`, `prepend` adds "Move to"), `link.rs`, `watch.rs` (+ `notify_moved` after every move), `dnd.rs`,
  `system.rs` (system icons: Recycle Bin, This PC, User's Files, Network, Control Panel as `::{CLSID}` items,
  name/icon from the shell, desktop visibility via HideDesktopIcons)
  (`drag_out` = OLE drag via SHDoDragDrop with our IDropSource: a drop on the desktop can be cancelled
  for Explorer and handled by the app; the app calls OleInitialize). The keeper never calls Explorer while a mouse button is down.
  E2E helpers shared by the examples live in `examples/support`.
- `src/fence/` fence window: `mod.rs` proc, `layout.rs` geometry, `items.rs` icons, `paint.rs` + `header.rs` rendering,
  `tabs.rs` tabs/portals, `anim.rs` unroll + chameleon fade, `peek.rs` hover, `menu.rs`, `actions.rs`, `title.rs` rename,
  `drop.rs` (from outside / onto a tab header / reorder inside a tab), `target.rs` (IDropTarget; an item under the cursor
  that accepts drops gets it through its own shell IDropTarget; drags of items already in fences only go into folders), `select.rs` (click,
  Ctrl+click, rubber band, drag out), `arrange.rs` (per-tab sort and rules submenus), `browse.rs` (folders opened inside
  the fence: `View.browse` stack, `browse::tab` swaps the active tab for a portal-like view used by loader/drop/paste, back via
  a click on the left third of the bar ("← folder" is always left-aligned there) or Backspace; a rolled fence goes `home` once
  it has rolled up; the fence menu's "Open folders ▸ In the fence / In File Explorer" sets `look.browse`, a folder's own
  right-click menu starts with "Open in the fence"), `moveto.rs` ("Move to" submenu of an item's menu), `group.rs` (group submenu; mates follow
  WM_MOVING via `grab`/`follow`/`release` and are left out of snapping), `desks.rs` ("Show on" virtual desktop submenu), `titlebar.rs` (title bar submenu),
  `wallpaper.rs` (fence color from the wallpaper behind it: `PaintDesktop` into a DIB, cached per rect in `View.wall`, reset on
  WM_SETTINGCHANGE/SPI_SETDESKWALLPAPER), `nav.rs` (arrow/Home/End/Page keys through `grid::step`, Ctrl+Tab), `system.rs` (system icons placement:
  menu, drag, drop), `window.rs` (create, owner/z-order: `raise` for peek), `host.rs` (hidden topmost window that owns
  every fence popup menu, so submenus stay above topmost windows; it forwards owner-draw menu messages to `native`), `snap.rs` (move/size snapping, Alt = free) + `guides.rs` (alignment lines: thin popups owned by the
  dragged fence). The drag anchor is taken on WM_NCLBUTTONDOWN: WM_ENTERSIZEMOVE only arrives after the first mouse move.
- System icons: `domain/system.rs` (`Sys`, pure) lives in `Tab.system` (legacy `recycle: true` migrates in `Config::sanitized`);
  `app/desk_icons.rs` hides them on the desktop while placed and restores the original HideDesktopIcons value
  (`Config.desk_icons`) when removed; dropping one on the desktop shows it. E2E: `cargo run --example system_e2e`.
- `src/visibility/`: `mod.rs` hide/show everything (fences + desktop listview, `rescue` from the panic hook) and peek
  (fences unowned + topmost via `fence::raise`, `overlay.rs` dim layer, ends on Esc, outside click or foreground change),
  `dblclick.rs` (WH_MOUSE_LL double-click on the empty desktop, item hit test through `Desk::item_at` once buttons are up),
  `hotkeys.rs` (RegisterHotKey on the tray, capture from the settings rows), `desks.rs` (virtual desktops, only while a fence
  is bound to a desktop: `shell::vwatch` waits on RegNotifyChangeKeyValue in a thread and posts `WM_DESKS`; 400 ms polling is
  the fallback if the watch cannot start; shows/hides fences; `show_all` and peek respect it). The double-click hook runs on its own thread
  (a blocked UI thread would make Windows drop a low-level hook). Startup re-shows the desktop icon list if a previous run died hidden. Typing in a fence while peeking opens `src/search/`
  (`hits.rs` collects items from every view and tab cache, `paint.rs`, topmost layered window, closes on deactivate).
- `src/prompt.rs`: one-line text prompt (`ask(initial, done)`), owned by the tray window (EN_KILLFOCUS commits), used to rename layouts.
- `src/settings/` settings window (custom drawn, sidebar of `pages::PAGES` (7 pages); each row can carry a `desc` explanation
  (2 lines max: row heights are fixed) and a `reset` link drawn on its own line, `layout::on_reset` hit test): `rows.rs` `Row`/`Act`/`Ctl`
  + row builders, `pages.rs` content per page (`build(a, page)`, window height = tallest page), `rules.rs` name/age rule editing
  (prompt + menus), `layout.rs` pure row geometry, `paint.rs`, `act.rs`, `mod.rs` window.
- Layout E2E: `cargo build && cargo run --example layout_e2e [-- --shots <dir>] [--desktop]` (startup scaling, snapping +
  guides, grid resize, drop onto a folder; `--desktop` also checks a new file under a fence on the real Desktop).
  Shared example helpers: `examples/support` (`app` launches the debug exe isolated, `wait`/`waited`/`poll`, windows,
  capture), `desk.rs` (desktop view), `input.rs` (SendInput mouse/keyboard). Peek/search/quick hide E2E:
  `cargo run --example peek_e2e [-- --shots <dir>]` (presses keys; uses Win+D when no desktop spot is visible).
  Menus: `cargo run --example menu_e2e [-- --shots <dir>]` (fence menu, submenu and an item's Windows menu stay above a
  topmost window). Features: `cargo run --example features_e2e [-- --shots <dir>]` (age rule, grouped fences moving
  together, folder opened inside a fence and Backspace, every settings page). `actions_e2e` (every fence menu command,
  Ctrl+wheel, Move to, paste, Shift+Delete, search Enter, Ctrl+double-click, title-bar back arrow, reorder, drag between fences,
  rubber band, tab click), `settings_e2e` (every settings row through clicks, shortcut capture/reset, rule editing, export/import
  with debug `RDI_PICK=<file>` replacing the file dialogs), `tray_e2e` (tray menu through `WM_TRAY`, layouts),
  `undo_e2e` (undo of moves, Move to, fence deletion with its file, rename via Ctrl+Z, duplicate, tray Undo),
  `display_e2e` (thumbnails, title alignment/size, bar on hover, wallpaper color, filtered portal, arrow keys, Ctrl+Tab; reads the
  `RDI_DUMP` canvases, `KEEP_DUMPS=<dir>` keeps them), `rules_e2e` (name/kind/"Everything else" rules and Sort now on the real Desktop, `*.rdie2e` files only; Sort now is tested
  with a name rule only so no user file can move), `vdesk_e2e` (temporary virtual desktop, closed at the end), `uninstall_e2e`,
  `recover_e2e` (orphan fence folders come back in a "Recovered files" fence, damaged config restored from `.bak`, BOM config),
  `soak_e2e` (`SOAK_MIN` minutes of everyday actions; GDI/USER/handles/private bytes must stay flat: `support::usage`),
  `load_e2e` (41 fences, 3,800 files: time to draw, memory, idle CPU). Debug builds tag every layered window with the
  `RDI_DRAWN` prop (last present had visible pixels); `support::fences` only lists drawn fences, so every E2E fails on a fence
  that never paints. Installer: `rdi-sandbox/bundle/msi.ps1` (Sandbox: MSI 0.9.4 -> current while running -> uninstall).
  What the Sandbox cannot test (several monitors, mixed DPI, Windows 10) is listed in `docs/manual-tests.md`.
  Menu helpers: `examples/support/menu.rs` finds entries by text (`pick(&["Behavior", "Lock position"])`, `checked`: our
  menus show checks as a trailing "	✓"); `clip.rs` puts files on the clipboard. E2E runs move the real mouse: any user
  input meanwhile (even a screenshot) makes them fail.
- `installer/main.wxs` per-user MSI (installs to `%LOCALAPPDATA%\Programs`, owns the HKCU Run value).

## Conventions
- User wants terse code: no comments, compact formatting (`rustfmt.toml` max_width 140).
- Business rules go in `domain.rs` with tests; Win32 code stays thin.
- Global state is `thread_local RefCell` accessed only via `app::with` (try_borrow). Never call Win32 APIs that
  send messages (DestroyWindow, SetWindowPos, TrackPopupMenu, MessageBox, CreateWindow) inside `with`.
- New UI string: add a `T` variant and translate it in all 25 rows of `i18n.rs` (tests check completeness).

## Robustness rules
- Never ignore a user-visible failure: collect errors and call `report::failures`/`report::alert` outside `app::with`.
  Yes/no questions use `report::confirm(h, T)`. After changing `Config.look`, call `fence::restyle_all()`.
- `store::save` writes `config.json.tmp` + `sync_all` + rename and keeps `config.json.bak`; an unreadable config (`Config::parse`:
  JSON object only, UTF-8 BOM accepted) is renamed `config.json.bad-<ts>` and never overwritten. `sanitized` never lets two tabs share an id (= a folder).
- Items keep a system image-list index (`Item.icon: i32`), never an HICON: icons are drawn on demand via
  `shell::draw_icon`. Folder reads + icon lookups run on a thread (`fence/loader.rs`, per-tab generations, one load at a
  time per fence: requests made meanwhile are queued in `View.queued`, a panicking load still posts an empty batch);
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
  `PREPARE` rebuilds it 150 ms after invalidation. `anim::start` does nothing when the fence is already at its targets
  (a plain fence no longer re-renders on mouse enter/leave), a reload whose items did not change skips `layout::apply`, drag-over
  checks each hovered item once per drag (`Target.checked`), `layered::measure` reuses one DC per thread, the system locale is read once.
  Label cache: names are ~70% of a fence render, so while the mouse is inside a fence `View.labels` keeps the label mask and
  its blur (`layered::Labels`, 2 bytes/px) keyed by size, body, scroll, cell/icon/scale/font and the visible names+positions
  (`paint::labels_key`); hover/selection renders reuse it (`Frame::layers`, test `cached_labels_paint_the_same_pixels`).
  Icons draw straight from the system image list (`IImageList::Draw`, no HICON),
  blur works on u8 buffers, hot paths read the fence via `read(h, ..)` instead of cloning it. `Canvas::rrect` fast-paths straight rows and `blur` only processes rows with text (tests prove equality).

## Gotchas
- Debug builds use a separate single-instance mutex, so they run beside the installed app. For visual checks, run the debug exe
  with `LOCALAPPDATA` pointed at a scratch dir (never touch the user's real data) and `RDI_DUMP=<file>` to dump the canvas.
- Background blur was tried and rejected (v0.3): DWM system backdrops go solid on inactive windows, and the
  undocumented SetWindowCompositionAttribute accent rendered opaque black with GDI content. Keep the layered path.
- Debug `RDI_TOP=1` makes fences topmost and unowned so screenshots can capture them.
- A new window starts at the top of the z-order without WM_WINDOWPOSCHANGING, so `fence::create` calls `raise(h, top())` after
  showing it (otherwise a new fence covers open windows until something moves it).
- Auto height: the last row counts its real content (`cell::content`: icon + measured names), plus a bottom gap equal to the
  top one (`layout::tail`); `grid::extent`/`max_scroll` use the same tail so there is no leftover scroll. Hover/selection tiles hug their item
  (`cell::tile`, names measured only for highlighted items) so they never run past a short last row.
- The active tab is saved with the next config save (at the latest on exit), not on every switch.
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
