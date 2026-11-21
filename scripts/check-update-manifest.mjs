// Checks the update manifest (`latest.json`) of a release before it is
// published: the app's updater reads it from the newest published release,
// so a manifest without a platform would leave that platform without updates.
// Fails if the version does not match the tag, the notes do not start with the
// tag's CHANGELOG section, or a platform has no download URL or signature.
// Usage: node scripts/check-update-manifest.mjs path/to/latest.json v0.15.0
import { readFileSync } from 'node:fs';

const [file, tag] = process.argv.slice(2);
if (!file || !tag) {
  console.error('Usage: node scripts/check-update-manifest.mjs latest.json vX.Y.Z');
  process.exit(2);
}

/** Platforms that must be in the manifest: an exact key, or a prefix one key must have. */
const REQUIRED = [
  { name: 'Windows', matches: (key) => key === 'windows-x86_64' },
  { name: 'macOS', matches: (key) => key.startsWith('darwin-') },
];

/** The heading `## vX.Y.Z – date` of `tag`, the date optional. */
const headingOf = (tag) => new RegExp(`^## ${tag.replace(/\./gu, '\\.')}(?:\\s.*)?$`, 'u');

const manifest = JSON.parse(readFileSync(file, 'utf8'));
const errors = [];
if (manifest.version !== tag.replace(/^v/u, '')) {
  errors.push(`version is ${manifest.version}, expected ${tag.replace(/^v/u, '')}`);
}
if (typeof manifest.notes !== 'string' || manifest.notes.trim() === '') {
  errors.push('has no release notes');
} else if (!headingOf(tag).test(manifest.notes.trimStart().split(/\r?\n/u)[0])) {
  // The app shows the sections newer than the installed version, found by their headings.
  errors.push(`notes do not start with the heading of ${tag}`);
}
const platforms = Object.entries(manifest.platforms ?? {});
for (const [key, entry] of platforms) {
  if (!entry?.url || !entry?.signature) errors.push(`${key} has no url or signature`);
}
for (const { name, matches } of REQUIRED) {
  if (!platforms.some(([key]) => matches(key))) errors.push(`has no ${name} entry`);
}

if (errors.length > 0) {
  errors.forEach((error) => console.error(`latest.json ${error}`));
  process.exit(1);
}
console.log(`latest.json for ${tag} lists ${platforms.map(([key]) => key).join(', ')}`);
