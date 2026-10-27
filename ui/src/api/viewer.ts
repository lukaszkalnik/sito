import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Which photos the viewer steps through; mirrors the timeline view. */
export type ViewMode = 'series' | 'all';

/** What the viewer shows. Mirrors `ViewerRequest` in `src-tauri/src/commands/viewer.rs`. */
export interface ViewerRequest {
  /** Timeline id of the photo. */
  id: number;
  scanId: number;
  /** View of the main window, which previous/next follow. */
  mode: ViewMode;
  /** Time gap of the series, in seconds. */
  gap: number;
}

/** The main window's view, sent to an open viewer when it changes. */
export interface ViewerContext {
  mode: ViewMode;
  gap: number;
}

const VIEWER_LABEL = 'viewer';
const SHOW_EVENT = 'viewer-show';
const CONTEXT_EVENT = 'viewer-context';

/** Shows a photo in the viewer window, opening the window if needed. */
export async function openViewer(request: ViewerRequest): Promise<void> {
  await invoke('open_viewer', { request });
}

/** The request a freshly opened viewer window should show, if any. */
export async function takeViewerRequest(): Promise<ViewerRequest | null> {
  return invoke<ViewerRequest | null>('take_viewer_request');
}

/** Calls `handler` for every photo the main window asks the viewer to show. */
export async function onViewerShow(handler: (request: ViewerRequest) => void): Promise<UnlistenFn> {
  return listen<ViewerRequest>(SHOW_EVENT, (event) => handler(event.payload));
}

/** Tells an open viewer window that the main window's view changed. */
export async function sendViewerContext(context: ViewerContext): Promise<void> {
  await emitTo(VIEWER_LABEL, CONTEXT_EVENT, context);
}

/** Calls `handler` whenever the main window's view changes. */
export async function onViewerContext(
  handler: (context: ViewerContext) => void,
): Promise<UnlistenFn> {
  return listen<ViewerContext>(CONTEXT_EVENT, (event) => handler(event.payload));
}

/** URL of the original file of timeline photo `id`, served by the `photo://` protocol. */
export function photoUrl(scanId: number, id: number): string {
  return `${convertFileSrc(String(id), 'photo')}?scan=${scanId}`;
}

/** Sets the title of the current window. */
export async function setWindowTitle(title: string): Promise<void> {
  await getCurrentWindow().setTitle(title);
}

/** Closes the current window. */
export async function closeWindow(): Promise<void> {
  await getCurrentWindow().close();
}
