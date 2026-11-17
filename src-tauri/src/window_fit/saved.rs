//! Reads the remembered frame of a window from the window-state plugin's file.
//!
//! The plugin keeps its state private; only the file's name is public. Its
//! format is pinned by `Cargo.lock`, and a file this module cannot read leaves
//! the window at its default place (with a warning).

use std::collections::HashMap;
use std::io::ErrorKind;

use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_window_state::AppHandleExt;

use super::{Point, Saved, Size};

/// One window's entry in the plugin's file, with only the fields used here,
/// all in physical pixels.
#[derive(Debug, Deserialize)]
struct Entry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    /// The position before the window was maximized.
    prev_x: i32,
    prev_y: i32,
    maximized: bool,
}

impl Entry {
    /// The remembered frame, or `None` for the plugin's placeholder of a window
    /// it has not seen yet (a zero size).
    fn saved(&self) -> Option<Saved> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        let position = if self.maximized {
            Point {
                x: self.prev_x,
                y: self.prev_y,
            }
        } else {
            Point {
                x: self.x,
                y: self.y,
            }
        };
        Some(Saved {
            position,
            inner_size: Size {
                width: self.width,
                height: self.height,
            },
            maximized: self.maximized,
        })
    }
}

/// The remembered frame of window `label`, or `None` if nothing is remembered.
/// A file that cannot be read is logged as a warning and counts as nothing
/// remembered; a missing one (first launch) is not a warning.
pub fn load<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<Saved> {
    let path = match app.path().app_config_dir() {
        Ok(dir) => dir.join(app.filename()),
        Err(err) => {
            log::warn!("no app config folder for the window state: {err}");
            return None;
        }
    };
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(err) if err.kind() == ErrorKind::NotFound => return None,
        Err(err) => {
            log::warn!("cannot read {}: {err}", path.display());
            return None;
        }
    };
    parse(&json, label).unwrap_or_else(|err| {
        log::warn!("cannot parse {}: {err}", path.display());
        None
    })
}

/// The remembered frame of window `label` in the plugin's file `json`.
fn parse(json: &str, label: &str) -> Result<Option<Saved>, serde_json::Error> {
    let entries: HashMap<String, Entry> = serde_json::from_str(json)?;
    Ok(entries.get(label).and_then(Entry::saved))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as the plugin writes it: a normal main window, a maximized viewer.
    const FILE: &str = r#"{
      "main": {
        "width": 920, "height": 620, "x": 20, "y": 40,
        "prev_x": 0, "prev_y": 0, "maximized": false,
        "visible": true, "decorated": true, "fullscreen": false
      },
      "viewer": {
        "width": 1200, "height": 800, "x": 0, "y": 60,
        "prev_x": 300, "prev_y": 180, "maximized": true,
        "visible": true, "decorated": true, "fullscreen": false
      }
    }"#;

    #[test]
    fn reads_a_window_from_the_plugins_file() {
        assert_eq!(
            parse(FILE, "main").ok().flatten(),
            Some(Saved {
                position: Point { x: 20, y: 40 },
                inner_size: Size {
                    width: 920,
                    height: 620
                },
                maximized: false,
            })
        );
    }

    #[test]
    fn a_maximized_window_keeps_its_position_from_before() {
        let viewer = parse(FILE, "viewer").ok().flatten();
        assert_eq!(viewer.map(|s| s.position), Some(Point { x: 300, y: 180 }));
        assert_eq!(viewer.map(|s| s.maximized), Some(true));
    }

    #[test]
    fn a_window_not_in_the_file_has_nothing_remembered() {
        assert_eq!(parse(FILE, "other").ok().flatten(), None);
    }

    #[test]
    fn the_plugins_placeholder_has_nothing_remembered() {
        let placeholder = r#"{"main": {
          "width": 0, "height": 0, "x": 0, "y": 0, "prev_x": 0, "prev_y": 0,
          "maximized": false, "visible": true, "decorated": true, "fullscreen": false
        }}"#;
        assert_eq!(parse(placeholder, "main").ok().flatten(), None);
    }

    #[test]
    fn a_broken_file_is_an_error() {
        assert!(parse("{\"main\": {\"width\": 920", "main").is_err());
        assert!(parse("{\"main\": {\"width\": -1}}", "main").is_err());
    }
}
