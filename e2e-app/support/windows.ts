import { browser } from '@wdio/globals';

/** A window's outer frame on screen, in physical pixels. */
export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** The app's two windows, by their Tauri labels. */
export type WindowLabel = 'main' | 'viewer';

/**
 * Calls a command of Tauri's window plugin from the current page, as the
 * app's own window API does. It acts on the native window, which is what the
 * window-state plugin saves.
 */
async function windowCommand<T>(command: string, args: Record<string, unknown>): Promise<T> {
  return (await browser.execute(
    async (name, payload) => {
      const tauri = (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__;
      return tauri.invoke(`plugin:window|${name}`, payload);
    },
    command,
    args,
  )) as T;
}

/** The outer frame of window `label`. */
export async function windowRect(label: WindowLabel): Promise<Rect> {
  const position = await windowCommand<{ x: number; y: number }>('outer_position', { label });
  const size = await windowCommand<{ width: number; height: number }>('outer_size', { label });
  return { ...position, ...size };
}

/**
 * Moves window `label` to `x`, `y` and gives its content `width` × `height`,
 * all in logical pixels; returns where the window ended up, which the system
 * may adjust (frames, menu bar).
 */
export async function placeWindow(
  label: WindowLabel,
  place: { x: number; y: number; width: number; height: number },
): Promise<Rect> {
  const { x, y, width, height } = place;
  await windowCommand('set_size', { label, value: { Logical: { width, height } } });
  await windowCommand('set_position', { label, value: { Logical: { x, y } } });
  return windowRect(label);
}

/**
 * Asks window `label` to close, like its close button does (the app saves
 * the window state on that request). The caller checks that it closed: when
 * it is the last window, the app may quit before WebDriver gets the reply,
 * which is then lost.
 */
export async function requestClose(label: WindowLabel): Promise<void> {
  await browser
    .execute((name) => {
      const tauri = (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__;
      setTimeout(() => void tauri.invoke('plugin:window|close', { label: name }), 0);
    }, label)
    .catch(() => {});
}
