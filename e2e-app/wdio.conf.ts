import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { browser } from '@wdio/globals';

import { appEnvironment, moveStateInto } from './support/appState.ts';
import { APP, DRIVER_PORT, copyApp, startDriver, stopDriver } from './support/driver.ts';

/** One folder per run, shared by the worker processes through the environment. */
const RUN = (process.env.SITO_E2E_RUN ??= new Date().toISOString().replace(/[:.]/g, '-'));
const RESULTS = join(fileURLToPath(new URL('./results/', import.meta.url)), RUN);

/** The spec this worker runs: its results folder and its app's environment. */
let current: { results: string; env: Readonly<Record<string, string>> } | null = null;

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./specs/**/*.spec.ts'],
  // One app at a time: the specs share the app's state folders, and the dev
  // machine has little memory.
  maxInstances: 1,
  hostname: '127.0.0.1',
  port: DRIVER_PORT,
  capabilities: [
    {
      // Read by tauri-driver, which starts this app; the embedded driver is the app.
      'tauri:options': { application: APP },
    } as WebdriverIO.Capabilities,
  ],
  logLevel: 'warn',
  waitforTimeout: 10_000,
  connectionRetryTimeout: 90_000,
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60_000 },
  reporters: ['spec'],

  onPrepare() {
    copyApp();
  },

  // Each spec starts from fresh state: whatever an earlier run left goes into
  // the results, and so does the spec's own state afterwards.
  async beforeSession(_config, _capabilities, specs) {
    const results = join(RESULTS, basename(specs[0] ?? 'spec', '.spec.ts'));
    const env = appEnvironment();
    moveStateInto(env, join(results, 'state-before'));
    current = { results, env };
    await startDriver(env, join(results, 'driver.log'));
  },

  async afterTest(test, _context, { passed }) {
    if (passed || current === null) return;
    await browser.saveScreenshot(join(current.results, `${test.title}.png`)).catch(() => {});
  },

  async afterSession() {
    await stopDriver();
    if (current !== null) moveStateInto(current.env, join(current.results, 'state'));
    current = null;
  },
};
