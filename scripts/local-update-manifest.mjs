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

const signature = readFileSync(`${installer}.sig`, 'utf8').trim();
const manifest = {
  version,
  notes: `Local test update to ${version}.`,
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
