// Prints the release notes of a version tag: its section of CHANGELOG.md
// followed by the installation notes. Fails if CHANGELOG.md has no section
// for the tag, so a release is never published without a changelog.
// Usage (from the repo root): node scripts/release-notes.mjs v0.13.0
import { readFileSync } from 'node:fs';

const tag = process.argv[2];
if (!tag) {
  console.error('Usage: node scripts/release-notes.mjs vX.Y.Z');
  process.exit(2);
}

const lines = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8').split(/\r?\n/u);
// The heading is `## vX.Y.Z – date`; `## v0.1.1` must not match `## v0.1.10`.
const isHeading = (line) => line.startsWith('## ');
const start = lines.findIndex((line) => line === `## ${tag}` || line.startsWith(`## ${tag} `));
const end = start < 0 ? -1 : lines.findIndex((line, index) => index > start && isHeading(line));
const section = start < 0 ? '' : lines.slice(start + 1, end < 0 ? undefined : end).join('\n').trim();

if (!section) {
  console.error(`CHANGELOG.md has no section for ${tag}`);
  process.exit(1);
}

const install = `## Installing on Windows

Download \`Sito_*_x64-setup.exe\` below and run it; it updates an installed Sito in place.
The installer is not signed yet, so Windows may show „System Windows ochronił ten komputer”
("Windows protected your PC"): click „Więcej informacji” (More info), then „Uruchom mimo to” (Run anyway).`;

console.log(`${section}\n\n${install}`);
