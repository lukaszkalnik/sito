/**
 * Pure logic of the third-party notices: which npm package a bundled module
 * belongs to, which licenses are allowed, and the text of the combined
 * `THIRD-PARTY-NOTICES.txt` (Sito's license, Rust crates, npm packages).
 */

/** An npm package bundled into the UI, with its license text. */
export interface NpmPackage {
  readonly name: string;
  readonly version: string;
  /** SPDX expression from `package.json`, if it has one. */
  readonly license: string | null;
  /** Contents of its license file(s); empty if it ships none. */
  readonly licenseText: string;
}

/** A license text of Sito itself. */
export interface OwnLicense {
  readonly title: string;
  readonly text: string;
}

/** Everything that goes into the notices file. */
export interface Notices {
  readonly sito: readonly OwnLicense[];
  /** Output of `npm run notices:rust`. */
  readonly rust: string;
  readonly npm: readonly NpmPackage[];
}

/**
 * Licenses npm packages in the UI bundle may use. Add one only after checking
 * that its terms allow shipping it in the installer.
 */
export const ALLOWED_NPM_LICENSES: ReadonlySet<string> = new Set(['MIT', 'ISC', 'Apache-2.0']);

const NODE_MODULES = '/node_modules/';
const RULE = '='.repeat(78);
const THIN_RULE = '-'.repeat(78);

/**
 * The folder of the npm package a bundled module comes from, with `/`
 * separators, or `null` for Sito's own sources and virtual modules.
 */
export function packageDirOf(moduleId: string): string | null {
  if (moduleId.startsWith('\0')) return null;
  const path = moduleId.replaceAll('\\', '/').split('?')[0];
  const start = path.lastIndexOf(NODE_MODULES);
  if (start < 0) return null;
  const rest = path.slice(start + NODE_MODULES.length).split('/');
  const nameParts = rest[0].startsWith('@') ? 2 : 1;
  if (rest.length <= nameParts) return null;
  return path.slice(0, start + NODE_MODULES.length) + rest.slice(0, nameParts).join('/');
}

/**
 * Whether an SPDX `expression` lets us use the package under an `allowed`
 * license: one of them, or a choice (`A OR B`) that includes one. Anything
 * more complex (`AND`, `WITH`) must be on the list as a whole.
 */
export function isAllowedLicense(expression: string, allowed: ReadonlySet<string>): boolean {
  if (allowed.has(expression)) return true;
  if (/\b(AND|WITH)\b/.test(expression)) return false;
  return expression
    .replace(/^\((.*)\)$/, '$1')
    .split(' OR ')
    .some((choice) => allowed.has(choice.trim()));
}

/**
 * Why `packages` cannot be shipped: one message per package whose license is
 * missing, not on `allowed`, or without a license text. Empty if all are fine.
 */
export function licenseProblems(
  packages: readonly NpmPackage[],
  allowed: ReadonlySet<string>,
): string[] {
  return packages.flatMap((pkg) => {
    const id = `${pkg.name}@${pkg.version}`;
    if (pkg.license === null) return [`${id} declares no license`];
    if (!isAllowedLicense(pkg.license, allowed))
      return [`${id} uses the unreviewed license ${pkg.license}`];
    if (pkg.licenseText.trim() === '') return [`${id} ships no license file`];
    return [];
  });
}

/** Converts Windows line endings, so the file is the same on every machine. */
function normalize(text: string): string {
  return text.replaceAll('\r\n', '\n').trim();
}

function section(title: string, body: string): string {
  return `${RULE}\n${title}\n${RULE}\n\n${body}\n`;
}

function sitoSection(licenses: readonly OwnLicense[]): string {
  const texts = licenses.map((l) => `${THIN_RULE}\n${normalize(l.text)}`);
  return section(
    'SITO',
    `Sito is licensed under the ${licenses.map((l) => l.title).join(' or the ')}, at your option.\n\n${texts.join('\n\n')}`,
  );
}

function npmEntry(pkg: NpmPackage): string {
  return [
    THIN_RULE,
    `${pkg.name} ${pkg.version} (${pkg.license ?? 'no license'})`,
    `https://www.npmjs.com/package/${pkg.name}/v/${pkg.version}`,
    '',
    normalize(pkg.licenseText),
  ].join('\n');
}

function npmSection(packages: readonly NpmPackage[]): string {
  const sorted = [...packages].sort((a, b) =>
    a.name === b.name ? a.version.localeCompare(b.version) : a.name.localeCompare(b.name),
  );
  return section(
    'NPM PACKAGES',
    `The following npm packages are bundled into Sito's user interface.\n\n${sorted.map(npmEntry).join('\n\n')}`,
  );
}

/** The text of `THIRD-PARTY-NOTICES.txt`. */
export function formatNotices(notices: Notices): string {
  return [
    'THIRD-PARTY NOTICES',
    '',
    "Sito's own license, then the licenses of the Rust crates and npm packages it contains.",
    '',
    sitoSection(notices.sito),
    section('RUST CRATES', normalize(notices.rust)),
    npmSection(notices.npm),
  ].join('\n');
}
