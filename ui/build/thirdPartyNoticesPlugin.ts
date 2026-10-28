import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import type { Plugin } from 'vite';

import {
  ALLOWED_NPM_LICENSES,
  formatNotices,
  licenseProblems,
  packageDirOf,
  type NpmPackage,
} from './thirdPartyNotices.ts';

/** Name of the notices file in the build output. */
export const NOTICES_FILE = 'THIRD-PARTY-NOTICES.txt';

const LICENSE_FILE = /^(licen[cs]e|copying)/i;

interface PluginOptions {
  /** Repository root, which holds Sito's `LICENSE-MIT` and `LICENSE-APACHE`. */
  readonly repoRoot: string;
  /** Output of `npm run notices:rust`. */
  readonly rustNotices: string;
}

interface PackageJson {
  readonly name?: unknown;
  readonly version?: unknown;
  readonly license?: unknown;
}

/** Reads the package in `dir`: its `package.json` and all its license files. */
function readPackage(dir: string): NpmPackage {
  const json = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8')) as PackageJson;
  const licenseText = readdirSync(dir)
    .filter((file) => LICENSE_FILE.test(file))
    .sort()
    .map((file) => readFileSync(join(dir, file), 'utf8'))
    .join('\n\n');
  return {
    name: String(json.name),
    version: String(json.version),
    license: typeof json.license === 'string' ? json.license : null,
    licenseText,
  };
}

/**
 * Writes `THIRD-PARTY-NOTICES.txt` into the build output: Sito's license, the
 * Rust notices and the licenses of exactly the npm packages in the bundle.
 * Fails the build if a bundled package has an unreviewed or no license.
 */
export function thirdPartyNotices(options: PluginOptions): Plugin {
  return {
    name: 'sito-third-party-notices',
    apply: 'build',
    generateBundle(_output, bundle) {
      const dirs = new Set(
        Object.values(bundle)
          .flatMap((file) => (file.type === 'chunk' ? file.moduleIds : []))
          .map(packageDirOf)
          .filter((dir) => dir !== null),
      );
      const npm = [...dirs].map(readPackage);
      const problems = licenseProblems(npm, ALLOWED_NPM_LICENSES);
      if (problems.length > 0) {
        this.error(`Third-party licenses need a review:\n${problems.join('\n')}`);
      }
      const sito = [
        { title: 'MIT License', text: readFileSync(join(options.repoRoot, 'LICENSE-MIT'), 'utf8') },
        {
          title: 'Apache License 2.0',
          text: readFileSync(join(options.repoRoot, 'LICENSE-APACHE'), 'utf8'),
        },
      ];
      this.emitFile({
        type: 'asset',
        fileName: NOTICES_FILE,
        source: formatNotices({ sito, rust: options.rustNotices, npm }),
      });
    },
  };
}
