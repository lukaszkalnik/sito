import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

import { thirdPartyNotices } from './build/thirdPartyNoticesPlugin.ts';

const repoRoot = fileURLToPath(new URL('..', import.meta.url));

// Tauri expects a fixed dev server port and needs the TAURI_ env prefix.
// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [
    react(),
    thirdPartyNotices({
      repoRoot,
      rustNotices: readFileSync(new URL('notices/rust-crates.txt', import.meta.url), 'utf8'),
    }),
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: ['es2022', 'chrome105', 'safari15'],
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.test.{ts,tsx}', 'build/**/*.test.ts'],
  },
});
