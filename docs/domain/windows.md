# Windows

Where Sito's two windows, the main window and the viewer, open.

## Remembered size and place

- Both windows remember their size and position across restarts, and whether they were maximized (on macOS: zoomed). The state is saved whenever a window closes, so also when the viewer is closed and reopened in the same run, and before an update's installer closes Sito.
- A window opens hidden and is shown once placed, so it never visibly jumps.
- A window comes back at its remembered size and position. A maximized one comes back maximized and, when un-maximized, returns to the frame it had before.
- A window that would lie outside the visible part of its screen (the work area, without taskbar, dock and menu bar) is moved into it, and shrunk where it is larger. A window reaching only a few pixels past an edge (the invisible resize borders on Windows) is left where it is.
- A window stays on the screen it was on (the one it overlaps most). One remembered on a screen that is no longer connected comes back on the primary screen.

## First launch

- With nothing remembered (first launch, or the state file removed or unreadable), the main window opens centred at its default size, and the viewer likewise; on a screen smaller than that, the window is shrunk into the work area.
