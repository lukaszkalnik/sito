//! Places windows when they open: at their remembered size and position, and
//! inside the visible part of the screen (the monitor's work area, which
//! excludes the taskbar), even when they were remembered on a larger or
//! disconnected monitor.

use tauri::{Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow, Window, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

/// What the window-state plugin remembers of each window.
pub const STATE_FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

/// How far (physical pixels) a window may reach outside the work area before
/// it is moved. Windows 10/11 frames have invisible resize borders of about
/// 7 px per side (more on high-DPI screens), so a window placed against a screen
/// edge measures slightly larger than what is visible.
const BORDER_SLACK: i64 = 16;

/// Restores `window`'s remembered size and position, keeps it on screen, and
/// shows it. The window must have been created hidden, so it never visibly jumps.
pub fn show_placed<R: Runtime>(window: &WebviewWindow<R>) {
    if let Err(err) = window.restore_state(STATE_FLAGS) {
        log::warn!("cannot restore the {} window's size: {err}", window.label());
    }
    if let Err(err) = fit_to_screen(window) {
        log::warn!(
            "cannot fit the {} window to the screen: {err}",
            window.label()
        );
    }
    if let Err(err) = window.show() {
        log::warn!("cannot show the {} window: {err}", window.label());
    }
}

/// Writes the size and position of all windows to disk when one is about to
/// close. The plugin itself only writes on app exit, which closing the last
/// window on Windows does not reliably reach, so the state was never saved.
pub fn save_on_close<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { .. } = event
        && let Err(err) = window.app_handle().save_window_state(STATE_FLAGS)
    {
        log::warn!("cannot save the window size and position: {err}");
    }
}

/// Whether `window` lies within `area`, allowing it to reach `slack` pixels
/// past each edge.
#[must_use]
pub fn fits_within(window: Rect, area: Rect, slack: i64) -> bool {
    let (wx, wy) = (i64::from(window.x), i64::from(window.y));
    let (ax, ay) = (i64::from(area.x), i64::from(area.y));
    wx >= ax - slack
        && wy >= ay - slack
        && wx + i64::from(window.width) <= ax + i64::from(area.width) + slack
        && wy + i64::from(window.height) <= ay + i64::from(area.height) + slack
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// `window` shrunk to fit into `area` and moved inside it.
#[must_use]
pub fn fit_into(window: Rect, area: Rect) -> Rect {
    let width = window.width.min(area.width);
    let height = window.height.min(area.height);
    Rect {
        x: clamp_start(window.x, width, area.x, area.width),
        y: clamp_start(window.y, height, area.y, area.height),
        width,
        height,
    }
}

/// Start of a span of `len` moved so it lies within `[area_start, area_start + area_len]`.
fn clamp_start(start: i32, len: u32, area_start: i32, area_len: u32) -> i32 {
    let room = i32::try_from(area_len - len).unwrap_or(i32::MAX);
    start.clamp(area_start, area_start.saturating_add(room))
}

/// Shrinks and moves `window` so it lies within the work area of its monitor
/// (or the primary monitor if it is on none). Maximized windows and windows
/// that only overlap by their invisible borders are left alone.
fn fit_to_screen<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    if window.is_maximized()? {
        return Ok(());
    }
    let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
        return Ok(());
    };
    let work = monitor.work_area();
    let area = Rect {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
    };
    let position = window.outer_position()?;
    let outer = window.outer_size()?;
    let current = Rect {
        x: position.x,
        y: position.y,
        width: outer.width,
        height: outer.height,
    };
    if fits_within(current, area, BORDER_SLACK) {
        return Ok(());
    }
    let fitted = fit_into(current, area);
    // `set_size` sets the inner size, so keep the frame's extra size out of it.
    let inner = window.inner_size()?;
    let frame_width = outer.width.saturating_sub(inner.width);
    let frame_height = outer.height.saturating_sub(inner.height);
    window.set_size(PhysicalSize::new(
        fitted.width.saturating_sub(frame_width),
        fitted.height.saturating_sub(frame_height),
    ))?;
    window.set_position(PhysicalPosition::new(fitted.x, fitted.y))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1920×1080 screen at 150 % with a 48 px taskbar at the bottom.
    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1032,
    };

    #[test]
    fn a_window_against_the_edges_fits_despite_its_invisible_borders() {
        // Snapped to the left half: the frame reaches 7 px past the left and
        // bottom edges of the work area.
        let window = Rect {
            x: -7,
            y: 0,
            width: 974,
            height: 1039,
        };
        assert!(fits_within(window, AREA, BORDER_SLACK));
    }

    #[test]
    fn a_window_clearly_outside_does_not_fit() {
        let below_taskbar = Rect {
            x: 100,
            y: 400,
            width: 1200,
            height: 800,
        };
        let on_another_monitor = Rect {
            x: 2500,
            y: 0,
            width: 800,
            height: 600,
        };
        assert!(!fits_within(below_taskbar, AREA, BORDER_SLACK));
        assert!(!fits_within(on_another_monitor, AREA, BORDER_SLACK));
    }

    #[test]
    fn leaves_a_window_that_fits() {
        let window = Rect {
            x: 100,
            y: 50,
            width: 1200,
            height: 800,
        };
        assert_eq!(fit_into(window, AREA), window);
    }

    #[test]
    fn shrinks_a_window_taller_than_the_screen_and_moves_it_to_the_top() {
        let window = Rect {
            x: 200,
            y: 120,
            width: 1920,
            height: 1200,
        };
        assert_eq!(
            fit_into(window, AREA),
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1032
            }
        );
    }

    #[test]
    fn moves_a_window_hanging_below_the_taskbar_back_up() {
        let window = Rect {
            x: 300,
            y: 400,
            width: 1200,
            height: 800,
        };
        assert_eq!(fit_into(window, AREA).y, 232);
        assert_eq!(fit_into(window, AREA).height, 800);
    }

    #[test]
    fn brings_back_a_window_from_a_disconnected_monitor() {
        let window = Rect {
            x: 2500,
            y: -300,
            width: 1000,
            height: 700,
        };
        assert_eq!(
            fit_into(window, AREA),
            Rect {
                x: 920,
                y: 0,
                width: 1000,
                height: 700
            }
        );
    }

    #[test]
    fn respects_a_work_area_that_does_not_start_at_zero() {
        // Second monitor to the left, taskbar at the top.
        let area = Rect {
            x: -1280,
            y: 40,
            width: 1280,
            height: 984,
        };
        let window = Rect {
            x: -1400,
            y: 0,
            width: 1280,
            height: 800,
        };
        assert_eq!(
            fit_into(window, area),
            Rect {
                x: -1280,
                y: 40,
                width: 1280,
                height: 800
            }
        );
    }
}
