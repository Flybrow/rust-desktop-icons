## What's new in Rust Desktop Icons 1.2.0

### New

- **Choose how each tab shows its items, like in File Explorer:** right-click a fence, open the new View menu and pick Icons, List, Details, Tiles or Content. The choice applies to the current tab only, so each tab of a fence can have its own view. This menu replaces "Show as a list" in the Appearance menu; Settings → Icons → "List view by default" still sets the view of tabs that have none.
- **Details view with columns:** each item takes one row with its name, date modified, type and size, as in File Explorer. Click a column title to sort the items by that column; click it again to reverse the order.
- **Tiles view:** a large icon with the name, the type and the size of the item written beside it.
- **Content view:** one wide row per item, with the name and the type on the left and the date modified and the size on the right.
- **Sort by date created:** a new choice in the fence menu → Sort by, next to name, type, date modified and size.

### Improvements

- **Smoother roll up and unroll:** the animation of a rolled fence now runs at the refresh rate of your screen, with evenly spaced frames. The small jolt at the start of each unroll is gone.
- **The magnifier of a portal now sits at the end of its own tab:** in a fence with several tabs, it used to sit at the far end of the title bar, over another tab, even though it filters the active tab.

### Fixes

- **A rolled fence stays open when you open a smaller folder inside it:** the fence used to shrink, leave the mouse outside, then roll up and go back to its own items.
- **No more crash with a very narrow side-bar fence:** a fence with its title bar on the side, shown as a list and made very narrow, could close the app.
- **A settings file that cannot be read is never overwritten:** if the file is locked by another program when the app starts, the app now says so and saves nothing until it is restarted, instead of replacing your layout.
- **"Start with Windows" and update messages tell the truth:** the option only stays ticked when Windows accepted it, and a failed update download now says so instead of "Could not check for updates".
- **Clearing "Filter files…" now shows every file again:** emptying the field and pressing Enter used to keep the old filter. The field also no longer starts with `*.pdf`, so opening it and clicking elsewhere no longer sets a filter by mistake.
