import { cpSync, existsSync, mkdirSync, mkdtempSync, renameSync, rmSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

/** The identifier of the e2e build (`src-tauri/tauri.e2e.conf.json`). */
export const IDENTIFIER = 'io.github.lukaszkalnik.sito.e2e';
/** The name the app under test runs as; macOS names an unbundled app's WebKit storage after it. */
export const EXECUTABLE = 'sito-e2e';

/** A folder the app under test keeps state in, with a name for its copy in the results. */
interface StateFolder {
  readonly name: string;
  readonly path: string;
}

/**
 * Environment of the app under test. On Linux its XDG folders (data with the
 * trash, config, cache) go to a new temp folder; elsewhere the identifier
 * alone keeps its state apart from the real Sito's.
 */
export function appEnvironment(): Readonly<Record<string, string>> {
  const env = {
    // The update check fails at once on a closed port, so no test reaches GitHub.
    SITO_UPDATE_ENDPOINT: 'http://127.0.0.1:9/latest.json',
    // The specs look for the Polish texts; the runners' systems are English.
    SITO_SYSTEM_LOCALE: 'pl-PL',
  };
  if (process.platform !== 'linux') return env;
  const root = mkdtempSync(join(tmpdir(), 'sito-e2e-state-'));
  return {
    ...env,
    XDG_DATA_HOME: join(root, 'data'),
    XDG_CONFIG_HOME: join(root, 'config'),
    XDG_CACHE_HOME: join(root, 'cache'),
  };
}

/** Every folder the app under test keeps state in, given its environment. */
function stateFolders(env: Readonly<Record<string, string>>): readonly StateFolder[] {
  const home = homedir();
  switch (process.platform) {
    case 'darwin':
      return [
        { name: 'app-support', path: join(home, 'Library/Application Support', IDENTIFIER) },
        { name: 'caches', path: join(home, 'Library/Caches', IDENTIFIER) },
        { name: 'logs', path: join(home, 'Library/Logs', IDENTIFIER) },
        { name: 'webkit', path: join(home, 'Library/WebKit', EXECUTABLE) },
        { name: 'webkit-caches', path: join(home, 'Library/Caches', EXECUTABLE) },
      ];
    case 'win32':
      return [
        { name: 'roaming', path: join(process.env.APPDATA ?? '', IDENTIFIER) },
        { name: 'local', path: join(process.env.LOCALAPPDATA ?? '', IDENTIFIER) },
      ];
    default:
      return (['XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME'] as const).map((key) => ({
        name: key.toLowerCase(),
        path: env[key] ?? '',
      }));
  }
}

/** The folder of the app's window state (`.window-state.json`). */
export function configFolder(env: Readonly<Record<string, string | undefined>>): string {
  switch (process.platform) {
    case 'darwin':
      return join(homedir(), 'Library/Application Support', IDENTIFIER);
    case 'win32':
      return join(process.env.APPDATA ?? '', IDENTIFIER);
    default:
      return join(env.XDG_CONFIG_HOME ?? '', IDENTIFIER);
  }
}

/**
 * Moves the app's state folders into `into`, so the next app starts fresh and
 * the state of a failed test stays for a look. Nothing is deleted.
 */
export async function moveStateInto(
  env: Readonly<Record<string, string>>,
  into: string,
): Promise<void> {
  for (const folder of stateFolders(env)) {
    if (folder.path === '' || !existsSync(folder.path)) continue;
    mkdirSync(into, { recursive: true });
    await moveSoon(folder.path, join(into, folder.name));
  }
}

const MOVE_ATTEMPTS = 40;
const MOVE_RETRY_MS = 250;

/**
 * Moves `from` to `to`, trying again for a while: on Windows the app's
 * WebView2 processes outlive it briefly and keep files open or changing.
 */
async function moveSoon(from: string, to: string): Promise<void> {
  for (let attempt = 1; ; attempt += 1) {
    try {
      move(from, to);
      return;
    } catch (err) {
      if (attempt >= MOVE_ATTEMPTS) throw err;
      await new Promise((resolve) => setTimeout(resolve, MOVE_RETRY_MS));
    }
  }
}

/** Moves `from` to `to`; across drives, by copying and then removing the original. */
function move(from: string, to: string): void {
  try {
    renameSync(from, to);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'EXDEV') throw err;
    cpSync(from, to, { recursive: true, force: true, preserveTimestamps: true });
    rmSync(from, { recursive: true });
  }
}
