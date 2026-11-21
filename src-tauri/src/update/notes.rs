//! The release notes sent to the UI with an update.
//!
//! A release's notes hold the CHANGELOG sections of several versions, each
//! under a `## vX.Y.Z – date` heading, so a user who skipped versions sees
//! what changed in all of them. Only the sections newer than the running
//! version are kept.

use semver::Version;

/// The start of a version heading.
const HEADING: &str = "## v";

/// `body` as the notes to show on `running`: the text before the first
/// version heading and every section whose version is newer than `running`,
/// trimmed. A section whose heading has no valid version is kept, and so is
/// every section if `running` is no valid version. `None` if nothing but
/// blank text is left.
pub fn notes_for(body: Option<&str>, running: &str) -> Option<String> {
    let running = Version::parse(running).ok();
    let kept: Vec<&str> = sections(body?)
        .filter(|section| is_newer(section, running.as_ref()))
        .collect();
    Some(kept.concat().trim().to_owned()).filter(|notes| !notes.is_empty())
}

/// `notes` cut before every line that starts a version heading, keeping the
/// line endings.
fn sections(notes: &str) -> impl Iterator<Item = &str> {
    let mut rest = notes;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = next_heading(rest).unwrap_or(rest.len());
        let (section, after) = rest.split_at(end);
        rest = after;
        Some(section)
    })
}

/// The byte offset of the first version heading in `text` after its first
/// line (a heading on the first line starts the section itself).
fn next_heading(text: &str) -> Option<usize> {
    text.match_indices('\n')
        .map(|(newline, _)| newline.saturating_add(1))
        .find(|&start| {
            text.get(start..)
                .is_some_and(|line| line.starts_with(HEADING))
        })
}

/// Whether `section` is to be shown on `running`: it is the text before the
/// first heading, its version is newer, or one of the versions is unknown.
fn is_newer(section: &str, running: Option<&Version>) -> bool {
    let (Some(version), Some(running)) = (version_of(section), running) else {
        return true;
    };
    version > *running
}

/// The version of the heading that starts `section`, if it has a valid one.
fn version_of(section: &str) -> Option<Version> {
    let heading = section.lines().next()?.strip_prefix(HEADING)?;
    let version = heading.split_whitespace().next()?;
    Version::parse(version).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: &str = "## v0.17.0 – 2026-12-01\n\n### New\n\n- Seventeen\n\n\
                         ## v0.16.0 – 2026-11-25\n\n### Fixed\n\n- Sixteen\n\n\
                         ## v0.15.1 – 2026-11-08\n\n### Fixed\n\n- Fifteen\n";

    fn notes(body: &str, running: &str) -> Option<String> {
        notes_for(Some(body), running)
    }

    #[test]
    fn blank_notes_are_no_notes() {
        assert_eq!(notes_for(None, "0.15.0"), None);
        assert_eq!(notes("", "0.15.0"), None);
        assert_eq!(notes("  \n", "0.15.0"), None);
    }

    #[test]
    fn notes_without_version_headings_are_kept_whole() {
        assert_eq!(
            notes("\n### New\n- Updates\n", "0.15.0").as_deref(),
            Some("### New\n- Updates")
        );
    }

    #[test]
    fn all_sections_are_kept_when_all_are_newer() {
        assert_eq!(notes(NOTES, "0.15.0").as_deref(), Some(NOTES.trim()));
    }

    #[test]
    fn sections_up_to_the_running_version_are_dropped() {
        let kept = notes(NOTES, "0.15.1");
        assert_eq!(
            kept.as_deref(),
            Some(
                "## v0.17.0 – 2026-12-01\n\n### New\n\n- Seventeen\n\n\
                 ## v0.16.0 – 2026-11-25\n\n### Fixed\n\n- Sixteen"
            )
        );
        assert_eq!(
            notes(NOTES, "0.16.5").as_deref(),
            Some("## v0.17.0 – 2026-12-01\n\n### New\n\n- Seventeen")
        );
    }

    #[test]
    fn nothing_is_left_when_no_section_is_newer() {
        assert_eq!(notes(NOTES, "0.17.0"), None);
    }

    #[test]
    fn a_prerelease_is_older_than_its_release() {
        assert!(notes(NOTES, "0.17.0-beta.1").is_some_and(|kept| kept.contains("Seventeen")));
    }

    #[test]
    fn text_before_the_first_heading_is_kept() {
        assert_eq!(
            notes("Intro\n## v0.15.1\n- Old\n", "0.16.0").as_deref(),
            Some("Intro")
        );
    }

    #[test]
    fn a_section_with_an_unknown_version_is_kept() {
        assert_eq!(
            notes("## v0.15.1\n- Old\n## vNext\n- New\n", "0.16.0").as_deref(),
            Some("## vNext\n- New")
        );
    }

    #[test]
    fn every_section_is_kept_when_the_running_version_is_unknown() {
        assert_eq!(notes(NOTES, "dev").as_deref(), Some(NOTES.trim()));
    }

    #[test]
    fn a_heading_inside_a_line_starts_no_section() {
        assert_eq!(
            notes("## v0.17.0\n- See ## v0.15.0 notes\n", "0.16.0").as_deref(),
            Some("## v0.17.0\n- See ## v0.15.0 notes")
        );
    }

    #[test]
    fn windows_line_endings_are_accepted() {
        assert_eq!(
            notes("## v0.17.0\r\n- New\r\n## v0.15.1\r\n- Old\r\n", "0.16.0").as_deref(),
            Some("## v0.17.0\r\n- New")
        );
    }
}
