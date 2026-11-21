## What's new in Rust Desktop Icons 1.1.0

### New

- **Icons hidden right inside File Explorer:** with "Keep files on the desktop", the icons you put in a fence are now hidden by File Explorer itself, like Fences does. They no longer flash after F5, a rename or a new file. If this cannot work on your PC, the previous method (one hidden cell in a corner) is used.
- **Filter a fence as you type:** press Ctrl+F in a fence, click the magnifier of a portal, or use the fence menu. A search field appears in the title bar and only the matching icons stay (accents ignored). Enter opens the first one, Escape closes the filter.
- **Privacy mode:** for screen sharing or people nearby, names become gray bars, icons plain squares and, if you want, titles dots. Choose when they come back: mouse over the fence, desktop in use, or never. Settings → Privacy, or the tray menu.
- **List view:** "Show as a list" puts one item per row, a small icon with its name beside it. Works for fences and portals.
- **Dark, light or automatic theme:** for the fences, menus, settings and dialogs. Automatic follows Windows. Colors you picked yourself are kept.
- **Rounded corners:** square, 4, 6 or 8 px, for every fence or only one.
- **Portal fences in one step:** "New portal fence…" in the tray menu and "Create an RDI portal fence…" in the desktop right-click menu create a fence that shows a folder.
- **Back to the start when clicking elsewhere:** a folder opened inside a fence can close by itself when you click outside the fence.
- **Hide all can leave the other desktop icons visible:** new option "Hide all: desktop icons too" in Settings → Desktop.

### Fixes

- **Sharp icons at every size:** icons resized with Ctrl+wheel or the icon size setting are no longer blurry or jagged between the sizes Windows provides.
- **Chinese, Japanese and Korean titles in a side bar** are stacked upright, top to bottom, instead of being turned sideways. Thanks to @asir66 for this contribution.
- **A rolled fence switched to a vertical title** is drawn correctly at once, not only after it has been unrolled.
- **Rolled fences keep their rounded corners** on all four sides.
- **Faster after an Explorer restart:** hidden icons show for a fraction of a second instead of up to 3 seconds.
