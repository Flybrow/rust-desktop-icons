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
- [ ] The desktop right-click entry "Create an RDI fence" works.

## Daily use for a day
- [ ] After a few hours: no lag when hovering fences; Task Manager shows memory roughly stable (well under 100 MB).
- [ ] Sleep / resume and lock / unlock: fences are still there and drawn.
