## What's new in Rust Desktop Icons 1.0.0

### New features
- **Peek:** press Ctrl+Alt+Space to bring every fence above your windows, on all monitors, with the rest of the screen dimmed. Esc, the same shortcut or a click outside brings everything back.
- **Search:** while peeking, just start typing to search every fence and every tab; Enter opens the item. Ctrl+Alt+F or the tray menu (Search…) opens the search directly, on the screen under the mouse.
- **Keyboard shortcuts:** a new Settings → Keyboard shortcuts page lets you change, turn off or reset the peek and search shortcuts, and set three more (none by default): hide or show everything, new fence, and open the settings. Shortcuts that Windows needs (Alt+F4, Alt+Tab, Ctrl+C…) are refused, and a shortcut already taken by another program is marked with ⚠.
- **Quick hide (off by default):** turn on Settings → Desktop → "Hide all on desktop double-click", then double-click an empty spot of the desktop to hide all fences and desktop icons; double-click again to bring them back. If the app is ever closed while everything is hidden, the icons come back at the next start.
- **Screen changes:** fences follow resolution and monitor changes. By default each screen setup keeps its own layout and new setups are scaled; you can also choose "Scale proportionally" or "Don't adjust" (Settings → Desktop → "When the screen changes").
- **Layouts:** save the current layout from the tray menu (Layouts → Save current layout), then restore, rename or delete it. A layout saved on another screen setup is scaled when restored.
- **Snapping:** a moved fence lines up with the edges of the other fences, with blue guide lines, and matches the spacing already used between them. Resizing snaps to whole rows and columns of icons. Hold Alt to move or resize freely.
- **New files never hide behind a fence:** a file saved on the desktop under a fence moves to a free visible spot.
- **Inbox:** the new "Everything else" rule sends every new desktop file that no other rule matches to the fence you choose.
- **Drop onto an item inside a fence:** drop a file on a folder to move it there, or on an app or shortcut to open the file with it. Items you drag from a fence only go into folders, so reordering never launches an app by mistake.
- **Compact view:** right-click a fence → Appearance → Show names. Without names, icons take less room and the hovered name shows in the title bar.
- **Icon alignment:** left (default), centered or right, for all fences in Settings → Default appearance, or for one fence from its right-click menu (Appearance → Icon alignment).
- **Sort by size:** next to name, type, date and your own order.
- **Open folders inside the fence:** double-click a folder in a fence to show its content right there; the arrow in the title bar (or Backspace) goes back. Ctrl+double-click opens Explorer. It can be turned off in Settings → Behavior, or per fence (right-click → Behavior).
- **Move to:** right-click an item → Move to, then pick any fence or tab, or the desktop.
- **Rules by name:** in Settings → Sorting rules, send new desktop files whose name matches a pattern (for example `invoice*.pdf`; `*` stands for any text, `?` for one character) to the fence you choose. Name rules are checked before file-type rules.
- **Rules by age:** items of a fence that have not been modified for 7 days to a year move automatically to another fence (Settings → Sorting rules → Add a rule by age). Checked a minute after start, then every hour.
- **Names and spacing:** choose the size (small, normal, large) and color (automatic or any color) of the names under the icons, and the spacing between icons (compact, normal, wide), in Settings → Icons or per fence (right-click → Appearance).
- **Undo (Ctrl+Z):** the last 20 actions can be undone: fences moved, resized, created, duplicated or deleted, their look and tabs, files moved in or out (drag and drop, Move to, automatic sorting, renaming) and settings. Press Ctrl+Z in a fence or in the settings, or use "Undo" at the top of a fence's right-click menu or of the tray menu.
- **Thumbnails:** pictures, videos and PDFs show a preview instead of their icon. On by default; turn them off in Settings → Icons or per fence (right-click → Appearance). They use a little more memory.
- **Filtered portals:** right-click a fence → Tabs → Filter files… to show only some files of a portal folder, for example `*.pdf` or `*.jpg; *.png`. Folders stay visible so you can still open them.
- **Keyboard navigation:** the arrow keys move between the icons of a fence (Shift adds to the selection), Home/End and Page Up/Down jump, Ctrl+Tab switches tabs.
- **Title bar options:** title alignment (left, centered, right), title size, a bar color of its own, and a bar that only shows when the mouse is over the fence. In the new Settings → Title bar page, or per fence (right-click → Appearance → Title bar).
- **Color from the wallpaper:** each fence can take a tone of the wallpaper behind it, updated when you move it or change the wallpaper (Settings → Default appearance, or right-click → Appearance).
- **Duplicate a fence:** right-click a fence → Duplicate creates a copy next to it with the same look, size and tabs (names, rules, sort, portal folders); the files themselves stay in the original.
- **Fence groups:** right-click a fence → Behavior → Group with, then pick other fences: moving one fence of the group moves the whole group. "Ungroup" takes a fence out of its group.
- **Virtual desktops:** right-click a fence → Behavior → Show on, to show it on every virtual desktop (default) or only on one. The menu appears once you have at least two virtual desktops.
- **Export and import:** save your fences, tabs, rules, layouts and settings to a file, and load them back on this PC or another one, from Settings → General. Your files are never part of the export; importing asks for confirmation and keeps the settings that belong to this PC (start with Windows, automatic updates, screen layouts, "Keep files on the desktop").

### Settings and menus
- **Settings apply to every fence:** changing a color, opacity, icon size, tint, names (shown, size, color), spacing, icon alignment, chameleon mode, automatic height or folder opening in Settings updates all fences right away. A setting you change on one fence from its right-click menu stays specific to that fence; right-click → Appearance → Reset to default makes it follow Settings again, and "Reset all fences to these settings" does it for every fence (after a confirmation). Existing fences keep the settings that differed from the defaults.
- **New settings window:** pages with a sidebar (Default appearance, Icons, Title bar, Behavior, Desktop, Sorting rules, Keyboard shortcuts, General), and a short explanation under every option.
- **Clearer right-click menu:** about half as long, with Appearance, Behavior and Tabs submenus. An item's right-click menu starts with "Move to".
- **Popups on the right screen:** settings, search and messages open on the screen under the mouse with the right size, also on monitors that use a different scaling.

### Fixes and reliability
- **Smoother:** hovering and selecting icons redraws a fence about a third faster (names are no longer redrawn when only the highlight changes), a fence no longer redraws when the mouse just enters or leaves it, and virtual desktop switches are picked up instantly instead of being checked several times a second.
- **Automatic height:** the space under the last row of icons now matches the space above the first row, whatever the icon size, name size or spacing. The highlight around a hovered or selected icon now fits its icon and name, and stays inside the fence.
- **New fences stay behind windows:** a fence created while windows are open (shortcut, tray menu) no longer appears above them.
- **Menus on top:** the right-click menus of a fence and their submenus always open above other windows; a submenu could show up behind an "always on top" window.
- **Dragging:** a dragged fence no longer lags slightly behind the cursor.
- **No flashing icons:** with "Keep files on the desktop" on, hidden icons no longer flash when their stack moves to another cell.
- **Slow folders:** fences reload their content one load at a time, so a slow network folder can no longer pile up background work.
- **Safer config:** a config written by a newer version of the app no longer resets your fences: unknown values are simply ignored.
- **Leaks:** small memory and resource leaks fixed (context menus, drag images).
