// Renders src-tauri/icons/source/icon.svg to a 1024×1024 PNG using Playwright's Chromium.
// Usage (from the repo root): node scripts/render-icon.mjs
// Then: npx tauri icon src-tauri/icons/source/icon.png
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Playwright is a dev dependency of the ui workspace.
const require = createRequire(new URL('../ui/package.json', import.meta.url));
const { chromium } = require('@playwright/test');

const svgPath = fileURLToPath(new URL('../src-tauri/icons/source/icon.svg', import.meta.url));
const pngPath = fileURLToPath(new URL('../src-tauri/icons/source/icon.png', import.meta.url));

const svg = await readFile(svgPath, 'utf8');
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
  await page.setContent(
    `<html><body style="margin:0;background:transparent">${svg}</body></html>`,
  );
  await page.locator('svg').screenshot({ path: pngPath, omitBackground: true });
  console.log(`Wrote ${pngPath}`);
} finally {
  await browser.close();
}
