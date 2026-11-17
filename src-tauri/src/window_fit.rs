//! Places windows when they open: at their remembered size and position, and
//! inside the visible part of the screen (the monitor's work area, which
//! excludes the taskbar), even when they were remembered on a larger or
//! disconnected monitor.

mod saved;

use tauri::{Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow, Window, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// What the window-state plugin remembers of each window.
pub const STATE_FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

/// How far (physical pixels) a window may reach outside the work area before
/// it is moved. Windows 10/11 frames have invisible resize borders of about
/// 7 px per side (more on high-DPI screens), so a window placed against a screen
/// edge measures slightly larger than what is visible.
const BORDER_SLACK: i64 = 16;

/// Puts `window` at its remembered size and position, kept on screen, and
/// shows it. The window must have been created hidden, so it never visibly jumps.
///
/// The frame is worked out from the remembered state and set once; nothing is
/// read back from the window after a set, because on macOS the setters only
/// take effect later (they are dispatched to the main queue).
pub fn show_placed<R: Runtime>(window: &WebviewWindow<R>) {
    let placed = match saved::load(window.app_handle(), window.label()) {
        Some(saved) => place(window, saved),
        None => fit_to_screen(window),
    };
    if let Err(err) = placed {
        log::warn!("cannot place the {} window: {err}", window.label());
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

/// A point in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// The remembered frame of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Saved {
    /// Outer position (top-left corner of the frame); for a maximized window,
    /// the position before it was maximized.
    pub position: Point,
    /// Size of the window's content, without title bar and borders.
    pub inner_size: Size,
    pub maximized: bool,
}

/// A monitor's full area and its work area (without taskbar, dock, menu bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorArea {
    pub area: Rect,
    pub work_area: Rect,
}

/// Where to put a window: its outer position and inner size, and whether to
/// maximize it afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub position: Point,
    pub inner_size: Size,
    pub maximize: bool,
}

/// Where a window remembered as `saved` goes. `frame_extra` is its title bar
/// and borders (outer size minus inner size). The remembered frame stays on the
/// monitor it overlaps most (the `primary` one if it overlaps none) and is moved
/// and shrunk into that monitor's work area if it does not lie within it.
#[must_use]
pub fn placement(
    saved: Saved,
    frame_extra: Size,
    monitors: &[MonitorArea],
    primary: Option<MonitorArea>,
) -> Placement {
    let frame = Rect {
        x: saved.position.x,
        y: saved.position.y,
        width: saved.inner_size.width.saturating_add(frame_extra.width),
        height: saved.inner_size.height.saturating_add(frame_extra.height),
    };
    let placed = monitor_of(frame, monitors)
        .or(primary)
        .map_or(frame, |monitor| keep_within(frame, monitor.work_area));
    Placement {
        position: Point {
            x: placed.x,
            y: placed.y,
        },
        inner_size: Size {
            width: placed.width.saturating_sub(frame_extra.width),
            height: placed.height.saturating_sub(frame_extra.height),
        },
        maximize: saved.maximized,
    }
}

/// `frame` as it is if it lies within `area`, otherwise fitted into it.
fn keep_within(frame: Rect, area: Rect) -> Rect {
    if fits_within(frame, area, BORDER_SLACK) {
        frame
    } else {
        fit_into(frame, area)
    }
}

/// The monitor whose area `frame` overlaps most, if it overlaps any.
fn monitor_of(frame: Rect, monitors: &[MonitorArea]) -> Option<MonitorArea> {
    monitors
        .iter()
        .map(|monitor| (overlap(frame, monitor.area), *monitor))
        .filter(|(shared, _)| *shared > 0)
        .max_by_key(|(shared, _)| *shared)
        .map(|(_, monitor)| monitor)
}

/// The area (square pixels) that `a` and `b` share.
fn overlap(a: Rect, b: Rect) -> u64 {
    span_overlap(a.x, a.width, b.x, b.width) * span_overlap(a.y, a.height, b.y, b.height)
}

/// How long the spans `[a, a + a_len]` and `[b, b + b_len]` share.
fn span_overlap(a: i32, a_len: u32, b: i32, b_len: u32) -> u64 {
    let start = i64::from(a.max(b));
    let end = (i64::from(a) + i64::from(a_len)).min(i64::from(b) + i64::from(b_len));
    u64::try_from(end - start).unwrap_or(0)
}

impl From<tauri::PhysicalRect<i32, u32>> for Rect {
    fn from(rect: tauri::PhysicalRect<i32, u32>) -> Self {
        Self {
            x: rect.position.x,
            y: rect.position.y,
            width: rect.size.width,
            height: rect.size.height,
        }
    }
}

impl From<&tauri::Monitor> for MonitorArea {
    fn from(monitor: &tauri::Monitor) -> Self {
        Self {
            area: Rect {
                x: monitor.position().x,
                y: monitor.position().y,
                width: monitor.size().width,
                height: monitor.size().height,
            },
            work_area: Rect::from(*monitor.work_area()),
        }
    }
}

/// Sets `window` to its remembered frame, kept on screen, and maximizes it if
/// it was maximized. Reads the window only before setting anything.
fn place<R: Runtime>(window: &WebviewWindow<R>, saved: Saved) -> tauri::Result<()> {
    let frame_extra = frame_extra(window)?;
    let monitors: Vec<MonitorArea> = window
        .available_monitors()?
        .iter()
        .map(MonitorArea::from)
        .collect();
    let primary = window.primary_monitor()?.as_ref().map(MonitorArea::from);
    let placement = placement(saved, frame_extra, &monitors, primary);
    // Position before size, as the window-state plugin restores: this order
    // keeps the remembered place on macOS.
    window.set_position(PhysicalPosition::new(
        placement.position.x,
        placement.position.y,
    ))?;
    window.set_size(PhysicalSize::new(
        placement.inner_size.width,
        placement.inner_size.height,
    ))?;
    if placement.maximize {
        window.maximize()?;
    }
    Ok(())
}

/// The window's title bar and borders: its outer size minus its inner size.
fn frame_extra<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<Size> {
    let outer = window.outer_size()?;
    let inner = window.inner_size()?;
    Ok(Size {
        width: outer.width.saturating_sub(inner.width),
        height: outer.height.saturating_sub(inner.height),
    })
}

/// Shrinks and moves a window with nothing remembered (first launch) so it lies
/// within the work area of its monitor (or the primary monitor if it is on
/// none). Windows that only overlap by their invisible borders are left alone.
/// Nothing has been set on the window yet, so reading its frame is safe.
fn fit_to_screen<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()?.or(window.primary_monitor()?) else {
        return Ok(());
    };
    let area = Rect::from(*monitor.work_area());
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
    let extra = frame_extra(window)?;
    window.set_size(PhysicalSize::new(
        fitted.width.saturating_sub(extra.width),
        fitted.height.saturating_sub(extra.height),
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

    /// The macOS CI runner: a 1024×768 screen with the menu bar and the dock
    /// outside the work area.
    const RUNNER: MonitorArea = MonitorArea {
        area: Rect {
            x: 0,
            y: 0,
            width: 1024,
            height: 768,
        },
        work_area: Rect {
            x: 0,
            y: 31,
            width: 1024,
            height: 674,
        },
    };

    /// A title bar of 28 px and no side borders, as on macOS.
    const TITLE_BAR: Size = Size {
        width: 0,
        height: 28,
    };

    fn saved(x: i32, y: i32, width: u32, height: u32) -> Saved {
        Saved {
            position: Point { x, y },
            inner_size: Size { width, height },
            maximized: false,
        }
    }

    fn placed(x: i32, y: i32, width: u32, height: u32) -> Placement {
        Placement {
            position: Point { x, y },
            inner_size: Size { width, height },
            maximize: false,
        }
    }

    #[test]
    fn a_remembered_frame_that_fits_its_screen_stays_where_it_was() {
        let main = saved(20, 40, 920, 620);
        assert_eq!(
            placement(main, TITLE_BAR, &[RUNNER], Some(RUNNER)),
            placed(20, 40, 920, 620)
        );
    }

    #[test]
    fn a_frame_larger_than_the_work_area_is_shrunk_into_it_with_room_for_the_title_bar() {
        let large = saved(-128, -32, 1280, 800);
        assert_eq!(
            placement(large, TITLE_BAR, &[RUNNER], Some(RUNNER)),
            placed(0, 31, 1024, 646)
        );
    }

    #[test]
    fn a_frame_on_a_disconnected_monitor_comes_back_on_the_primary_one() {
        let gone = saved(2500, 100, 800, 600);
        assert_eq!(
            placement(gone, TITLE_BAR, &[RUNNER], Some(RUNNER)),
            placed(224, 77, 800, 600)
        );
    }

    #[test]
    fn a_frame_is_fitted_into_the_monitor_it_lies_on_not_the_primary_one() {
        // A second screen to the right, without a menu bar.
        let right = MonitorArea {
            area: Rect {
                x: 1024,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 1024,
                y: 0,
                width: 1920,
                height: 1080,
            },
        };
        // Mostly on the right screen, hanging below its bottom edge.
        let window = saved(1000, 600, 1200, 600);
        assert_eq!(
            placement(window, TITLE_BAR, &[RUNNER, right], Some(RUNNER)),
            placed(1024, 452, 1200, 600)
        );
    }

    #[test]
    fn a_maximized_window_is_placed_at_its_earlier_frame_and_maximized() {
        let window = Saved {
            maximized: true,
            ..saved(300, 180, 600, 420)
        };
        assert_eq!(
            placement(window, TITLE_BAR, &[RUNNER], Some(RUNNER)),
            Placement {
                maximize: true,
                ..placed(300, 180, 600, 420)
            }
        );
    }

    #[test]
    fn without_any_monitor_the_remembered_frame_is_kept() {
        let main = saved(-500, -500, 920, 620);
        assert_eq!(
            placement(main, TITLE_BAR, &[], None),
            placed(-500, -500, 920, 620)
        );
    }
}
