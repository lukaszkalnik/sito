//! Reading a Takeout date file.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// What Sito takes from a Takeout date file: the capture time and the photo's
/// original name. Every other field (places, descriptions, people) is never
/// read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    /// When the photo was taken, as Google Photos knows it.
    pub taken: DateTime<Utc>,
    /// The photo's file name when it was uploaded.
    pub title: Option<String>,
}

/// Why a date file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    /// The file could not be read.
    #[error("cannot read the date file")]
    Io(#[from] std::io::Error),
    /// The file is not a Takeout date file.
    #[error("not a Takeout date file")]
    Format(#[from] serde_json::Error),
    /// The file has no usable capture time (missing, zero or out of range).
    #[error("no capture time in the date file")]
    NoTime,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    title: Option<String>,
    photo_taken_time: Option<RawTime>,
}

#[derive(Deserialize)]
struct RawTime {
    /// Unix seconds, as a string.
    timestamp: String,
}

/// Reads the Takeout date file at `path`.
///
/// # Errors
///
/// Fails if the file cannot be read, is not JSON of the expected shape, or
/// has no capture time (Takeout writes `0` for unknown).
pub fn read_sidecar(path: &Path) -> Result<Sidecar, SidecarError> {
    parse_sidecar(&std::fs::read(path)?)
}

fn parse_sidecar(json: &[u8]) -> Result<Sidecar, SidecarError> {
    let raw: Raw = serde_json::from_slice(json)?;
    let taken = raw
        .photo_taken_time
        .and_then(|time| time.timestamp.trim().parse::<i64>().ok())
        .filter(|&seconds| seconds != 0)
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .ok_or(SidecarError::NoTime)?;
    Ok(Sidecar {
        taken,
        title: raw.title.filter(|title| !title.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_capture_time_and_title() {
        let json = br#"{
            "title": "IM000171.JPG",
            "description": "",
            "creationTime": { "timestamp": "1785091503", "formatted": "26 lip 2026, 18:45:03 UTC" },
            "photoTakenTime": { "timestamp": "1119834548", "formatted": "27 cze 2005, 01:09:08 UTC" },
            "geoData": { "latitude": 50.1, "longitude": 18.6, "altitude": 0.0 },
            "people": [{ "name": "Someone" }]
        }"#;
        let sidecar = parse_sidecar(json).unwrap();
        assert_eq!(sidecar.taken.to_rfc3339(), "2005-06-27T01:09:08+00:00");
        assert_eq!(sidecar.title.as_deref(), Some("IM000171.JPG"));
    }

    #[test]
    fn accepts_dates_before_1970() {
        let json = br#"{ "photoTakenTime": { "timestamp": "-157766400" } }"#;
        assert_eq!(
            parse_sidecar(json).unwrap().taken.to_rfc3339(),
            "1965-01-01T00:00:00+00:00"
        );
    }

    #[test]
    fn a_zero_or_missing_time_is_no_time() {
        for json in [
            &br#"{ "title": "a.jpg", "photoTakenTime": { "timestamp": "0" } }"#[..],
            br#"{ "title": "a.jpg" }"#,
            br#"{ "photoTakenTime": { "timestamp": "soon" } }"#,
        ] {
            assert!(matches!(parse_sidecar(json), Err(SidecarError::NoTime)));
        }
    }

    #[test]
    fn other_json_is_a_format_error() {
        assert!(matches!(
            parse_sidecar(b"[1, 2]"),
            Err(SidecarError::Format(_))
        ));
    }
}
