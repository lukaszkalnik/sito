//! Choosing between a photo's EXIF capture time and its Takeout date.

use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};

use crate::metadata::{CaptureTime, is_plausible_capture_time};

/// Largest UTC offset in use (UTC+14, Kiribati), in seconds.
const MAX_OFFSET_SECONDS: i64 = 14 * 3600;
/// Every time zone offset is a multiple of 15 minutes.
const OFFSET_STEP_SECONDS: i64 = 15 * 60;
/// Seconds the two times may differ beyond a whole time zone offset.
const TOLERANCE_SECONDS: i64 = 2;

/// Why a photo's time comes from Takeout instead of EXIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TakeoutUse {
    /// The photo has no EXIF capture time.
    MissingExif,
    /// The EXIF capture time is not plausible (e.g. a camera default).
    ImplausibleExif,
    /// The EXIF capture time differs by more than a time zone: the date was
    /// most likely corrected in Google Photos.
    Corrected,
}

/// The capture time chosen for a photo with a Takeout date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenTime {
    /// The EXIF capture time, which Takeout confirms.
    Exif(NaiveDateTime),
    /// The Takeout date, as local wall-clock time.
    Takeout(NaiveDateTime, TakeoutUse),
}

/// Chooses between the EXIF capture time and the Takeout date `takeout`.
///
/// EXIF times are local and Takeout dates are UTC, so a difference that is a
/// whole time zone offset (a multiple of 15 minutes within ±14 hours, give or
/// take two seconds) keeps the EXIF time. Any other difference means the date
/// was changed in Google Photos, and the Takeout date wins, as it does when
/// the EXIF time is missing or not plausible at `now`.
///
/// A Takeout date is shown in the EXIF offset when the photo has one, else
/// converted with `to_local` (the computer's time zone).
#[must_use]
pub fn choose_time(
    exif: Option<CaptureTime>,
    takeout: DateTime<Utc>,
    now: NaiveDateTime,
    to_local: impl Fn(DateTime<Utc>) -> NaiveDateTime,
) -> ChosenTime {
    let local = |offset: Option<chrono::FixedOffset>| match offset {
        Some(offset) => takeout.with_timezone(&offset).naive_local(),
        None => to_local(takeout),
    };
    match exif {
        None => ChosenTime::Takeout(local(None), TakeoutUse::MissingExif),
        Some(exif) if !is_plausible_capture_time(exif.local, now) => {
            ChosenTime::Takeout(local(exif.offset), TakeoutUse::ImplausibleExif)
        }
        Some(exif) if differs_by_time_zone(exif.local - takeout.naive_utc()) => {
            ChosenTime::Exif(exif.local)
        }
        Some(exif) => ChosenTime::Takeout(local(exif.offset), TakeoutUse::Corrected),
    }
}

fn differs_by_time_zone(difference: TimeDelta) -> bool {
    let seconds = difference.num_seconds();
    let off_step = seconds.rem_euclid(OFFSET_STEP_SECONDS);
    let near_step =
        off_step <= TOLERANCE_SECONDS || off_step >= OFFSET_STEP_SECONDS - TOLERANCE_SECONDS;
    seconds.abs() <= MAX_OFFSET_SECONDS + TOLERANCE_SECONDS && near_step
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;

    use super::*;

    fn at(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn utc(time: NaiveDateTime) -> DateTime<Utc> {
        time.and_utc()
    }

    fn exif(local: NaiveDateTime) -> CaptureTime {
        CaptureTime {
            local,
            offset: None,
        }
    }

    /// A fixed zone of UTC+2 instead of the computer's, so tests are deterministic.
    fn plus_two(time: DateTime<Utc>) -> NaiveDateTime {
        time.naive_utc() + TimeDelta::hours(2)
    }

    fn now() -> NaiveDateTime {
        at("2026-10-28 12:00:00")
    }

    fn choose(exif: impl Into<Option<CaptureTime>>, takeout: NaiveDateTime) -> ChosenTime {
        choose_time(exif.into(), utc(takeout), now(), plus_two)
    }

    #[test]
    fn without_exif_takes_takeout_in_local_time() {
        assert_eq!(
            choose(None, at("2024-06-01 10:00:05")),
            ChosenTime::Takeout(at("2024-06-01 12:00:05"), TakeoutUse::MissingExif)
        );
    }

    #[test]
    fn keeps_exif_when_only_the_time_zone_differs() {
        let local = at("2024-06-01 12:00:05");
        for takeout in [
            at("2024-06-01 10:00:05"), // UTC+2
            at("2024-06-01 12:00:05"), // UTC
            at("2024-06-01 06:30:05"), // UTC+5:30, India
            at("2024-06-01 06:15:06"), // UTC+5:45, Nepal, one second off
            at("2024-05-31 22:00:05"), // UTC+14
            at("2024-06-02 00:00:05"), // UTC-12
        ] {
            assert_eq!(
                choose(exif(local), takeout),
                ChosenTime::Exif(local),
                "{takeout}"
            );
        }
    }

    #[test]
    fn takes_takeout_when_the_date_was_corrected() {
        let corrected =
            |takeout| ChosenTime::Takeout(plus_two(utc(takeout)), TakeoutUse::Corrected);
        let local = at("2024-06-01 12:00:05");
        for takeout in [
            at("2024-06-01 10:07:05"), // 7 minutes off a zone
            at("2024-05-31 21:00:05"), // 15 hours
            at("2023-06-01 10:00:05"), // a year
        ] {
            assert_eq!(
                choose(exif(local), takeout),
                corrected(takeout),
                "{takeout}"
            );
        }
    }

    #[test]
    fn a_scan_dated_in_the_seventies_gets_its_takeout_date() {
        let scanned = at("2006-03-04 18:22:10");
        let set_in_google_photos = at("1976-07-01 10:00:00");
        assert_eq!(
            choose(exif(scanned), set_in_google_photos),
            ChosenTime::Takeout(at("1976-07-01 12:00:00"), TakeoutUse::Corrected)
        );
    }

    #[test]
    fn replaces_an_implausible_exif_time() {
        let takeout = at("2024-06-01 10:00:05");
        let expected = ChosenTime::Takeout(at("2024-06-01 12:00:05"), TakeoutUse::ImplausibleExif);
        for wrong in [at("2000-01-01 00:00:00"), at("2030-01-05 09:00:00")] {
            assert_eq!(choose(exif(wrong), takeout), expected, "{wrong}");
        }
    }

    #[test]
    fn shows_takeout_in_the_photos_own_offset() {
        let offset = FixedOffset::west_opt(5 * 3600).unwrap();
        let photo = Some(CaptureTime {
            local: at("2000-01-01 00:00:00"),
            offset: Some(offset),
        });
        assert_eq!(
            choose(photo, at("2024-06-01 17:00:00")),
            ChosenTime::Takeout(at("2024-06-01 12:00:00"), TakeoutUse::ImplausibleExif)
        );
    }
}
