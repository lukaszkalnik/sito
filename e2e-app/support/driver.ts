import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { copyFileSync, createWriteStream, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { EXECUTABLE } from './appState.ts';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const EXE = process.platform === 'win32' ? '.exe' : '';
/** The app built by `npm run e2e:app:build` (`:embedded` on Windows and macOS). */
const BUILT = join(ROOT, 'target/debug', `sito${EXE}`);
/** The copy of the app the tests run. */
export const APP = join(ROOT, 'target/e2e-app', `${EXECUTABLE}${EXE}`);

/**
 * How the tests reach the app: on Windows and macOS through the WebDriver
 * server the e2e build embeds (feature `e2e`), on Linux through `tauri-driver`
 * and WebKitWebDriver, which start the app for each session. macOS has no
 * WebDriver for WKWebView; on Windows, the Edge WebDriver could not attach to
 * the app's WebView2 (slice 28).
 */
const EMBEDDED = process.platform !== 'linux';
/** The port WebdriverIO connects to. */
export const DRIVER_PORT = EMBEDDED ? 4445 : 4444;
/** How long the driver or the app gets to come up. */
const START_TIMEOUT_MS = 60_000;
const POLL_MS = 100;

/** The driver process of this worker, the environment it gave the app, and its log. */
interface Running {
  readonly child: ChildProcess;
  readonly env: NodeJS.ProcessEnv;
  readonly log: string;
}

// On `globalThis`, as the config's hooks and the specs each load this module.
const store = globalThis as typeof globalThis & { sitoE2eDriver?: Running | null };
const running = (): Running | null => store.sitoE2eDriver ?? null;

/** Copies the built app to `APP`, under the name its state folders go by. */
export function copyApp(): void {
  mkdirSync(dirname(APP), { recursive: true });
  copyFileSync(BUILT, APP);
}

/** Starts the driver (or the app with its embedded one) with `appEnv`, logging to `log`. */
export async function startDriver(appEnv: Readonly<Record<string, string>>, log: string) {
  const env = { ...process.env, ...appEnv };
  store.sitoE2eDriver = { child: spawnLogged(env, log), env, log };
  // A worker whose session never starts gets no afterSession hook.
  process.once('exit', () => running()?.child.kill());
  await waitForStatus();
}

/** The environment the app under test runs with. */
export function appEnv(): NodeJS.ProcessEnv {
  const current = running();
  if (current === null) throw new Error('the driver is not running');
  return current.env;
}

/** Starts the app again after it has quit, with the same environment as before. */
export async function startAppAgain(): Promise<void> {
  const current = running();
  // tauri-driver starts the app for the next session by itself.
  if (!EMBEDDED || current === null) return;
  store.sitoE2eDriver = { ...current, child: spawnLogged(current.env, current.log) };
  await waitForStatus();
}

/** Stops the driver and makes sure no app under test is left running. */
export async function stopDriver(): Promise<void> {
  const child = running()?.child;
  child?.kill();
  // Until it has exited, Windows keeps its state folders locked.
  if (child !== undefined)
    await pollUntil(
      () => child.exitCode !== null || child.signalCode !== null,
      'the driver to stop',
    );
  store.sitoE2eDriver = null;
  if (appRunning()) killApp();
}

/** Waits until the app under test has quit, as it does when its last window closes. */
export async function waitForAppExit(): Promise<void> {
  await pollUntil(() => !appRunning(), 'the app to quit');
}

function spawnLogged(env: NodeJS.ProcessEnv, log: string): ChildProcess {
  mkdirSync(dirname(log), { recursive: true });
  const out = createWriteStream(log, { flags: 'a' });
  const child = EMBEDDED
    ? spawn(APP, [], { env: { ...env, TAURI_WEBDRIVER_PORT: String(DRIVER_PORT) } })
    : spawn('tauri-driver', driverArgs(), { env });
  child.stdout?.pipe(out);
  child.stderr?.pipe(out);
  return child;
}

/** `tauri-driver`'s arguments; `SITO_E2E_NATIVE_DRIVER` names the Edge or WebKit driver. */
function driverArgs(): string[] {
  const native = process.env.SITO_E2E_NATIVE_DRIVER;
  return ['--port', String(DRIVER_PORT), ...(native ? ['--native-driver', native] : [])];
}

/** Waits until the WebDriver server answers and, for the embedded one, its webview is up. */
async function waitForStatus(): Promise<void> {
  await pollUntil(async () => {
    try {
      const response = await fetch(`http://127.0.0.1:${DRIVER_PORT}/status`);
      const body = (await response.json()) as { value?: { ready?: boolean } };
      return body.value?.ready !== false;
    } catch {
      return false;
    }
  }, `the WebDriver server on port ${DRIVER_PORT}`);
}

async function pollUntil(done: () => boolean | Promise<boolean>, what: string): Promise<void> {
  const deadline = Date.now() + START_TIMEOUT_MS;
  while (!(await done())) {
    if (Date.now() > deadline) throw new Error(`Timed out waiting for ${what}`);
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
}

/** Whether a process of the app under test is running. */
function appRunning(): boolean {
  if (EMBEDDED) return running()?.child.exitCode === null;
  // Started by tauri-driver's native driver, on Linux.
  return spawnSync('pgrep', ['-f', APP]).status === 0;
}

function killApp(): void {
  spawnSync('pkill', ['-f', APP]);
}
