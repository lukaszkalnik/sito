// Prints the release notes of a version tag: its section of CHANGELOG.md
// followed by the installation notes. Fails if CHANGELOG.md has no section
// for the tag, so a release is never published without a changelog.
// With --update-notes, prints the notes the app shows with an update
// (`latest.json`) instead: the sections from the tag back to the first
// version with the updater, each with its `## vX.Y.Z – date` heading, so a
// user who skipped versions sees all of them (the app keeps those newer than
// the installed version). Installing by hand does not apply there.
// Usage (from the repo root): node scripts/release-notes.mjs v0.13.0 [--update-notes]
import { readFileSync } from 'node:fs';

const [tag, flag] = process.argv.slice(2);
const updateNotes = flag === '--update-notes';
if (!tag || (flag !== undefined && !updateNotes)) {
  console.error('Usage: node scripts/release-notes.mjs vX.Y.Z [--update-notes]');
  process.exit(2);
}

const lines = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8').split(/\r?\n/u);
// The heading is `## vX.Y.Z – date`; `## v0.1.1` must not match `## v0.1.10`.
const isHeading = (line) => line.startsWith('## ');
const isHeadingOf = (version) => (line) => line === `## ${version}` || line.startsWith(`## ${version} `);
/** The index of the heading after `index`, or `undefined` at the end of the file. */
const nextHeading = (index) => {
  const next = lines.findIndex((line, other) => other > index && isHeading(line));
  return next < 0 ? undefined : next;
};
const start = lines.findIndex(isHeadingOf(tag));
const end = start < 0 ? -1 : nextHeading(start) ?? -1;
const section = start < 0 ? '' : lines.slice(start + 1, end < 0 ? undefined : end).join('\n').trim();

if (!section) {
  console.error(`CHANGELOG.md has no section for ${tag}`);
  process.exit(1);
}

const install = `## Installing on Windows

Download \`Sito_*_x64-setup.exe\` below and run it; it updates an installed Sito in place.
The installer is not signed yet, so Windows may show „System Windows ochronił ten komputer”
("Windows protected your PC"): click „Więcej informacji” (More info), then „Uruchom mimo to” (Run anyway).

## Installing on macOS

Download \`Sito_*_universal.dmg\` below, open it and drag Sito into Applications.
The app is not signed yet, so macOS refuses the first start. Click Done, open
System Settings → Privacy & Security, scroll down to Security, click „Otwórz mimo to” (Open Anyway)
next to the note about Sito and confirm. A newly downloaded version may need this again.`;

/** The first version with the updater: no installed app is older. */
const FIRST_UPDATER = 'v0.15.0';

/**
 * The sections from the tag back to FIRST_UPDATER, with their headings; only
 * the tag's own for a tag older than FIRST_UPDATER.
 */
function sectionsSinceFirstUpdater() {
  const first = lines.findIndex((line, index) => index >= start && isHeadingOf(FIRST_UPDATER)(line));
  const last = nextHeading(first < 0 ? start : first);
  return lines.slice(start, last).join('\n').trim();
}

console.log(updateNotes ? sectionsSinceFirstUpdater() : `${section}\n\n${install}`);
