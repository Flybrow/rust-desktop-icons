# Manual checks before a release

Everything else is covered by `cargo test` and the E2E examples (run them in Windows Sandbox, see CLAUDE.md).
These need real hardware the Sandbox does not have. Tick them on the release build installed from the MSI.

## Several monitors and mixed scaling (e.g. a 100 % landscape screen + a 150 % portrait screen)
- [ ] A fence on each screen keeps its size and position after a restart.
- [ ] Dragging a fence from one screen to the other: icons and names change size once, nothing blurs, snapping guides line up.
- [ ] Settings, search, what's new and the update popup open on the screen under the mouse, at the right size; moving them
      to the other screen re-fits them (no cut-off text).
- [ ] Unplug the second screen: its fences come to the main screen. Plug it back: they return where they were.
- [ ] Change the resolution or the scaling of one screen: fences follow (Settings → Desktop → When the screen changes).
- [ ] Right-click menus and submenus of a fence on the second screen open above other windows.

## Windows 10 (22H2)
- [ ] Install, update from the previous version, uninstall: same results as `msi.ps1` in the Sandbox.
- [ ] Virtual desktops: "Show on" lists the desktops; switching desktops shows/hides the bound fences.
- [ ] The desktop right-click entries "Create an RDI fence" and "Create an RDI portal fence…" work.

## Look at 125 % or 150 % scaling
- [ ] Ctrl+wheel through every icon size: icons stay sharp (no blur, no jagged edges) at every step.
- [ ] Ctrl+F in a fence: the search field, its hint, the caret and ✕ are aligned in the title bar; the caret blinks.
- [ ] A rolled fence with Large corners: all four corners are round, nothing square shows behind them.

## Theme Automatic
- [ ] Switch Windows between light and dark mode (Settings → Personalization → Colors): the settings window, menus and
      fences that use the default color follow at once, without restarting the app.

## Privacy mode with two screens
- [ ] Reveal "When the desktop is in use": clicking the desktop of either screen shows the fences' content, clicking a window masks it again.

## Daily use for a day
- [ ] After a few hours: no lag when hovering fences; Task Manager shows memory roughly stable (well under 100 MB).
- [ ] Sleep / resume and lock / unlock: fences are still there and drawn.

## Keep files on the desktop (icons hidden inside Explorer)
- [ ] Windows 10: pinned icons disappear from the desktop (no stack in the bottom-right corner), F5 does not show them.
- [ ] Two screens: pinned icons placed on the second screen are hidden too; a loose icon dragged between screens never shows them.
- [ ] Microsoft Defender / SmartScreen does not flag the app or `%LOCALAPPDATA%\RustDesktopIcons\hook\rdi_hook-*.dll`.
- [ ] Update to a newer version while the mode is on: icons stay hidden; after the next Explorer restart only the new DLL is left in `hook\`.
- [ ] Windows on ARM: the mode still works through the stack fallback (no ARM64 DLL).

## Side bars with Chinese, Japanese or Korean titles (the Sandbox has no CJK font)
- [ ] A vertical fence titled e.g. "文件夹" shows upright characters, top to bottom, none overlapping; "Tools" stays rotated.
