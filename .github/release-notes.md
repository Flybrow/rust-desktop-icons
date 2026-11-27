## What's new in Rust Desktop Icons 1.3.0

### New

- **Glass theme:** fences can become sheets of frosted glass, with the wallpaper softly blurred behind them. No frame, a light title bar, and the active tab in a discreet pill. Turn it on in Settings → Default appearance → Theme → Glass, or for one fence only: right-click it → Appearance → Theme. The glass is dark or light depending on the fence color, and darkens or lightens just enough over a very bright or very dark wallpaper so that names stay readable. All your appearance settings still apply: color, opacity (how strongly the fence color tints the glass, from the bare wallpaper to an almost solid color), title bar color, corners, icon and name sizes, title alignment and size.
- **Mode replaces the old Theme setting:** choose dark, light or automatic (following Windows) on its own, now that Theme picks Default or Glass. It sits at the top of Settings → Default appearance, right under Theme.
- **Larger corners:** fences can now be rounded by 12 or 16 px. Pick them in Settings → Default appearance → Corners or in each fence's Appearance menu.
- **Import a backup made on another PC:** the layout and the settings come back without mixing that PC's files with yours. The files already in your fences stay where they are; any that no longer have a fence come back in a "Recovered files" fence.

### Improvements

- **A rolled fence takes the height of each tab you open:** switching to a tab with fewer items now shrinks the open fence to fit, and every fence glides to the height of the tab you pick instead of jumping. It still keeps its size when you open a smaller folder inside it.
- **Smoother rolled fences:** a fence always visibly unrolls, even when it has to be redrawn first, and rolls up as soon as the mouse leaves.
- **Fences stay on the desktop with Win+D on Windows 10:** "Show desktop" (Win+D or the corner of the taskbar) no longer hides the fences, including right after sign-in (issue #4).
- **Faster with big fences:** sorting a tab in your own order, searching every fence and "Keep files on the desktop" with thousands of files are much quicker.
- **Less memory for thumbnails:** a fence keeps the previews of its open tab only, within a fixed budget; beyond it, items show their usual icon.
- **Safer updates:** downloads are checked more strictly and only over HTTPS, slow connections no longer time out, and updating works when the Windows user name contains a "!".

### Fixes

- **Alt+F4 no longer closes a fence.**
- **"Wallpaper color" fences keep the same color on every tab.**
- **Delete while typing in a fence's filter no longer deletes the selected files.**
- **Renaming a tab always renames the tab you started on,** even if the mouse passes over another tab meanwhile.
- **Your own order is kept:** reordering icons while a filter is on, or picking a sort while a folder is open inside the fence, no longer drops the hidden items from your order.
- **Undo puts a file back on the desktop for good:** automatic sorting no longer moves it straight back into a fence.
- **Moving files never overwrites a file:** if a file with the same name appears meanwhile, the next free name is used. Between two drives, the original is only deleted once its copy is complete and unchanged.
- **Uninstalling never deletes your files:** if a fence folder cannot be read, nothing is deleted and you are told.
- **Keep files on the desktop:** a file that disappears for a moment (saved by Office, a network desktop that drops) stays in its fence, and an icon dragged out of a fence lands under the mouse.
- **One wrong value no longer resets your layout:** a damaged settings file now keeps every fence it can read.
- **Desktop icons you hid in File Explorer stay hidden** after a quick hide or a restart of the app.
- **Dropping a web link or text on a fence** now shows the "not allowed" cursor instead of doing nothing.
- **Settings:** a double-click on a row no longer acts twice.
- **Privacy mode:** "When the desktop is in use" now also counts a click on a fence.
- **New fences stay on the screen,** even when created near an edge.
- **The desktop right-click entries work when the app runs as administrator,** as do the tray icon and its return after an Explorer restart.
- **Many smaller fixes** found by a full code review: rare crashes, files shown in the wrong tab after a quick folder change, warnings repeated every hour for a locked file, virtual desktops no longer followed after an Explorer restart.
