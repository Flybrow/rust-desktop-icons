## What's new in Rust Desktop Icons 1.0.0

### New features
- **Peek:** press Ctrl+Alt+Space to bring every fence above your windows, on all monitors, with the rest of the screen dimmed. Esc, the same shortcut or a click outside brings everything back.
- **Search:** while peeking, just start typing to search every fence and every tab; Enter opens the item. Ctrl+Alt+F or the tray menu (Search…) opens the search directly, on the screen under the mouse.
- **Customizable shortcuts:** both shortcuts can be changed, turned off or reset to their default in Settings → Behavior. Shortcuts that Windows needs (Alt+F4, Alt+Tab, Ctrl+C…) are refused, and a shortcut already taken by another program is marked with ⚠.
- **Quick hide (off by default):** turn on Settings → Desktop → "Hide all on desktop double-click", then double-click an empty spot of the desktop to hide all fences and desktop icons; double-click again to bring them back. If the app is ever closed while everything is hidden, the icons come back at the next start.
- **Screen changes:** fences follow resolution and monitor changes. By default each screen setup keeps its own layout and new setups are scaled; you can also choose "Scale proportionally" or "Don't adjust" (Settings → Desktop → "When the screen changes").
- **Layouts:** save the current layout from the tray menu (Layouts → Save current layout), then restore, rename or delete it. A layout saved on another screen setup is scaled when restored.
- **Snapping:** a moved fence lines up with the edges of the other fences, with blue guide lines, and matches the spacing already used between them. Resizing snaps to whole rows and columns of icons. Hold Alt to move or resize freely.
- **New files never hide behind a fence:** a file saved on the desktop under a fence moves to a free visible spot.
- **Inbox:** the new "Everything else" rule sends every new desktop file that no other rule matches to the fence you choose.
- **Drop onto an item inside a fence:** drop a file on a folder to move it there, or on an app or shortcut to open the file with it. Items you drag from a fence only go into folders, so reordering never launches an app by mistake.
- **Compact view:** right-click a fence → Appearance → Show names. Without names, icons take less room and the hovered name shows in the title bar.
- **Icon alignment:** left (default), centered or right, for all fences in Settings → Default appearance, or for one fence from its right-click menu (Appearance → Icon alignment).
- **Sort by size**, next to name, type, date and your own order.

### Settings and menus
- **Settings apply to every fence:** changing a color, opacity, icon size, tint, names, icon alignment, chameleon mode or automatic height in Settings updates all fences right away. A setting you change on one fence from its right-click menu stays specific to that fence; right-click → Appearance → Reset to default makes it follow Settings again, and "Reset all fences to these settings" does it for every fence (after a confirmation). Existing fences keep the settings that differed from the defaults.
- **New settings window:** pages with a sidebar (Default appearance, Behavior, Desktop, Sorting rules, General), and a short explanation under every option.
- **Clearer right-click menu:** about half as long, with Appearance, Behavior and Tabs submenus.
- Popup windows (settings, search, messages) open on the screen under the mouse with the right size, also on monitors that use a different scaling.

### Fixes and reliability
- The right-click menus of a fence and their submenus always open above other windows; a submenu could show up behind an "always on top" window.
- A dragged fence no longer lags slightly behind the cursor.
- With "Keep files on the desktop" on, hidden icons no longer flash when their stack moves to another cell.
- Fences reload their content one load at a time, so a slow network folder can no longer pile up background work.
- A config written by a newer version of the app no longer resets your fences: unknown values are simply ignored.
- Small memory and resource leaks fixed (context menus, drag images).
