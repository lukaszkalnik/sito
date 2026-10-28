import { describe, expect, it } from 'vitest';

import {
  formatNotices,
  isAllowedLicense,
  licenseProblems,
  packageDirOf,
  type NpmPackage,
} from './thirdPartyNotices';

const allowed = new Set(['MIT', 'Apache-2.0']);

function pkg(overrides: Partial<NpmPackage> = {}): NpmPackage {
  return {
    name: 'react',
    version: '19.0.0',
    license: 'MIT',
    licenseText: 'MIT License',
    ...overrides,
  };
}

describe('packageDirOf', () => {
  it('finds the package of a bundled module', () => {
    expect(packageDirOf('/repo/node_modules/react/cjs/react.production.js')).toBe(
      '/repo/node_modules/react',
    );
  });

  it('keeps the scope of scoped packages and uses / on Windows', () => {
    expect(packageDirOf('D:\\sito\\node_modules\\@tauri-apps\\api\\core.js')).toBe(
      'D:/sito/node_modules/@tauri-apps/api',
    );
  });

  it('uses the innermost package of nested node_modules', () => {
    expect(packageDirOf('/r/node_modules/a/node_modules/b/index.js?commonjs-proxy')).toBe(
      '/r/node_modules/a/node_modules/b',
    );
  });

  it('ignores own sources and virtual modules', () => {
    expect(packageDirOf('/repo/ui/src/App.tsx')).toBeNull();
    expect(packageDirOf('\0vite/preload-helper.js')).toBeNull();
  });
});

describe('isAllowedLicense', () => {
  it('accepts an allowed license or a choice that includes one', () => {
    expect(isAllowedLicense('MIT', allowed)).toBe(true);
    expect(isAllowedLicense('Apache-2.0 OR MIT', allowed)).toBe(true);
    expect(isAllowedLicense('(GPL-3.0 OR MIT)', allowed)).toBe(true);
  });

  it('rejects other licenses and combinations not reviewed as a whole', () => {
    expect(isAllowedLicense('GPL-3.0', allowed)).toBe(false);
    expect(isAllowedLicense('MIT AND GPL-3.0', allowed)).toBe(false);
    expect(isAllowedLicense('Apache-2.0 WITH LLVM-exception', allowed)).toBe(false);
  });
});

describe('licenseProblems', () => {
  it('passes packages with an allowed license and its text', () => {
    expect(licenseProblems([pkg()], allowed)).toEqual([]);
  });

  it('reports a missing or unreviewed license and a missing license file', () => {
    expect(
      licenseProblems(
        [
          pkg({ name: 'a', license: null }),
          pkg({ name: 'b', license: 'GPL-3.0' }),
          pkg({ name: 'c', licenseText: '  ' }),
        ],
        allowed,
      ),
    ).toEqual([
      'a@19.0.0 declares no license',
      'b@19.0.0 uses the unreviewed license GPL-3.0',
      'c@19.0.0 ships no license file',
    ]);
  });
});

describe('formatNotices', () => {
  const text = formatNotices({
    sito: [
      { title: 'MIT License', text: 'MIT License\r\n\r\nCopyright (c) Sito' },
      { title: 'Apache License 2.0', text: 'Apache License' },
    ],
    rust: 'Used by:\n  image 0.25.10\n',
    npm: [
      pkg({ name: 'scheduler', version: '0.28.0', licenseText: 'scheduler license' }),
      pkg({ name: 'react', version: '19.3.1', licenseText: 'react license' }),
    ],
  });

  it("puts Sito's license first, then Rust crates, then npm packages", () => {
    const order = ['SITO', 'Copyright (c) Sito', 'RUST CRATES', 'image 0.25.10', 'NPM PACKAGES'];
    const positions = order.map((part) => text.indexOf(part));
    expect(positions.every((pos) => pos >= 0)).toBe(true);
    expect(positions).toEqual([...positions].sort((a, b) => a - b));
    expect(text).toContain('MIT License or the Apache License 2.0, at your option');
  });

  it('lists npm packages by name with version, license, link and full text', () => {
    expect(text.indexOf('react 19.3.1 (MIT)')).toBeLessThan(text.indexOf('scheduler 0.28.0'));
    expect(text).toContain(
      'react 19.3.1 (MIT)\nhttps://www.npmjs.com/package/react/v/19.3.1\n\nreact license',
    );
  });

  it('uses Unix line endings', () => {
    expect(text).not.toContain('\r');
  });
});
