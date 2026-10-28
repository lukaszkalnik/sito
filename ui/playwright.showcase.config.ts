import { defineConfig, devices } from '@playwright/test';

// README screenshots: the UI with mocked Tauri commands showing the demo gallery
// from scripts/make-demo-gallery.py. Kept apart from the e2e tests.
export default defineConfig({
  testDir: './showcase',
  forbidOnly: !!process.env.CI,
  use: {
    baseURL: 'http://localhost:5173',
    locale: 'pl-PL',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
  },
});
