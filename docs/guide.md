# User guide

Everything is also in the app: right-click a fence for its menu, or open **Settings** from the tray icon (every option has
a short explanation). Settings apply to every fence; a change made from one fence's menu stays specific to that fence,
and **Appearance → Reset to default** makes it follow Settings again.

## Fences

| Action | How |
| --- | --- |
| Create a fence | Tray icon → **New fence**, right-click the desktop → **Create an RDI fence**, or right-click a fence |
| Portal fence (shows a folder) | Tray icon → **New portal fence…**, or right-click the desktop → **Create an RDI portal fence…** |
| Rename | Right-click the title |
| Move / resize | Drag the title bar / an edge; fences snap to each other and to whole rows of icons (hold **Alt** to move freely) |
| Roll up / down | Double-click the title bar: a rolled fence shows only its title and opens on hover |
| Vertical fence | Right-click → **Behavior** → **Vertical title**, then park it on a screen edge |
| Tabs | Right-click → **Tabs**; **Ctrl+Tab** switches; **Filter files…** turns a portal tab into `*.pdf; *.jpg` only |
| Duplicate | Right-click → **Duplicate**: same look, size and tabs, without the files |
| Group fences | Right-click → **Behavior** → **Group with**: they move together |
| Virtual desktops | Right-click → **Behavior** → **Show on** → all desktops or one |
| Delete | Right-click → **Delete fence**: its items go back to the desktop |

## Icons

| Action | How |
| --- | --- |
| Add | Drag them onto a fence from the desktop, the Start menu or File Explorer |
| Open | Double-click; a folder opens inside the fence (title-bar arrow or **Backspace** to go back, **Ctrl+double-click** for File Explorer) |
| Move elsewhere | Drag it out, or right-click → **Move to** → a fence, a tab or the desktop |
| Delete | Select it and press **Delete** (Recycle Bin) |
| Keyboard | Arrow keys, Home/End, Page Up/Down, Shift to extend the selection |
| Undo | **Ctrl+Z**, or **Undo** at the top of a fence's or the tray icon's menu (last 20 actions) |
| Icon size | **Ctrl + mouse wheel** over a fence |
| Sort | Right-click → **Sort by** (name, type, dates, size or your own order) |
| Views | Right-click → **View** → Icons, List, Details, Tiles or Content (per tab); in Details, click a column title to sort, drag its edge to resize |
| System icons | Right-click → **System icons** (Recycle Bin, This PC, Network…) |

## Look

| Action | How |
| --- | --- |
| Theme | Settings → Default appearance → **Theme**: Default or **Glass** (frosted glass); per fence: right-click → **Appearance** → **Theme** |
| Dark / light | Settings → Default appearance → **Mode**: Dark, Light or Automatic (follows Windows) |
| Color, opacity, corners | Settings → Default appearance, or right-click → **Appearance**; **Color from the wallpaper** picks it for you |
| Names, spacing, alignment | Settings → Icons, or right-click → **Appearance** |
| Title bar | Settings → **Title bar** (alignment, size, color, show on hover) |
| Thumbnails | Pictures, videos and PDFs show a preview; turn off in Settings → Icons |

## Find and hide

| Action | How |
| --- | --- |
| Peek | **Ctrl+Alt+Space** brings every fence above your windows; **Esc** or a click outside ends it |
| Search every fence | Type while peeking, **Ctrl+Alt+F**, or tray icon → **Search…**; **Enter** opens the item |
| Filter one fence | **Ctrl+F** in it; **Esc** closes the filter |
| Hide everything | Settings → Desktop → **Hide all on desktop double-click**, then double-click an empty spot of the desktop |
| Privacy mode | Tray icon → **Privacy mode**: names, icons and titles are masked until you hover a fence |
| Shortcuts | Settings → **Keyboard shortcuts** |

## Organize automatically

| Action | How |
| --- | --- |
| Sorting rules | Right-click → **Sorting rules**: new desktop files of that kind (apps, games, images, documents…) go to this fence |
| Inbox | Right-click → **Sorting rules** → **Everything else** |
| Rules by name / age | Settings → **Sorting rules**: `invoice*.pdf` to a fence; items not modified for N days to an archive |

## Screens, layouts and backups

| Action | How |
| --- | --- |
| Screen changes | Each monitor setup keeps its own layout; a new setup is scaled automatically |
| Layouts | Tray icon → **Layouts** → **Save current layout**, then restore, rename or delete |
| Export / import | Settings → **General** → **Export the configuration…** / **Import a configuration…** (never your files) |

## Where your files are

By default, files from your desktop are **moved** into `%LOCALAPPDATA%\RustDesktopIcons\fences\<id>` (never deleted);
anything else gets a **shortcut**. Deleting a fence or uninstalling moves its items back to the desktop. If the
configuration is ever damaged, the files left in `fences\<id>` come back at the next start in a **Recovered files** fence.

With **Settings → Keep files on the desktop**, fenced files stay in your Desktop folder (visible in File Explorer) and
only their desktop icons are hidden. Turning it on or off moves the existing files accordingly.
