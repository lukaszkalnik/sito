import { getVersion } from '@tauri-apps/api/app';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Event of the menu item „Pomoc → Licencje…”. Mirrors `SHOW_LICENSES_EVENT` in `src-tauri/src/about.rs`. */
const SHOW_LICENSES_EVENT = 'about-show-licenses';
/** The notices file the frontend build writes (`ui/build/thirdPartyNoticesPlugin.ts`). */
export const NOTICES_URL = '/THIRD-PARTY-NOTICES.txt';

/** Calls `handler` whenever the user picks „Licencje…” in the menu. */
export async function onShowLicenses(handler: () => void): Promise<UnlistenFn> {
  return listen(SHOW_LICENSES_EVENT, () => handler());
}

/**
 * The text of the third-party notices.
 *
 * @throws if the file is missing; the dev server, which does not build it,
 * answers with its HTML page instead.
 */
export async function loadNotices(): Promise<string> {
  const response = await fetch(NOTICES_URL);
  const type = response.headers.get('content-type') ?? '';
  if (!response.ok || !type.startsWith('text/plain')) {
    throw new Error(`No notices at ${NOTICES_URL} (${response.status}, ${type})`);
  }
  return response.text();
}

/** The version of the running app. */
export async function appVersion(): Promise<string> {
  return getVersion();
}
