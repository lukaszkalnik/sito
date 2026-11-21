// Writes a `latest.json` next to a signed Windows installer, for testing an
// update without a release: serve the installer's folder on port 8000 and run
// a debug build with SITO_UPDATE_ENDPOINT=http://localhost:8000/latest.json.
// The installer needs its `.sig` file, which a build with the signing key
// writes next to it (the Release workflow's artifact carries both).
// Usage: node scripts/local-update-manifest.mjs path/to/Sito_0.15.0_x64-setup.exe 99.0.0
import { readFileSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';

const [installer, version] = process.argv.slice(2);
if (!installer || !version) {
  console.error('Usage: node scripts/local-update-manifest.mjs Sito_x64-setup.exe 99.0.0');
  process.exit(2);
}

// Two versions with every construct the in-app notes format, so a local test
// shows the formatting.
const notes = `## v${version} – 2026-12-01

### New

- **A local test update to ${version}.** It shows \`code\`, bold text and a link as its text. ([slice 54](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/54-release-notes-markdown.md))
- A second item.

## v0.0.1

### Fixed

- Shown only by a version older than 0.0.1, so never.`;

const signature = readFileSync(`${installer}.sig`, 'utf8').trim();
const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      url: `http://localhost:8000/${encodeURIComponent(basename(installer))}`,
      signature,
    },
  },
};
const out = join(dirname(installer), 'latest.json');
writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Wrote ${out}`);
