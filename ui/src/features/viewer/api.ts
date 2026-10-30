import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import type { TimelineEntry } from '../timeline/api';

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

/** One photo as the viewer shows it. Mirrors `ViewerPhotoDto`. */
export interface ViewerPhoto {
  readonly entry: TimelineEntry;
  /** Id of the previous photo of the view, `null` at the start. */
  readonly previous: number | null;
  /** Id of the next photo of the view, `null` at the end. */
  readonly next: number | null;
  /** The photos of its series, newest first (empty for a single photo). */
  readonly series: readonly TimelineEntry[];
  /** Id of the photo to show once this one is rejected, `null` if none is left. */
  readonly afterReject: number | null;
}

/**
 * What the viewer shows for `request`, or `null` if the photo is not on the
 * timeline of that scan (e.g. a restored photo that is not back yet).
 */
export async function getViewerPhoto(request: ViewerRequest): Promise<ViewerPhoto | null> {
  return invoke<ViewerPhoto | null>('get_viewer_photo', { request });
}

/** The main window's view, sent to an open viewer when it changes. */
export interface ViewerContext {
  mode: ViewMode;
  gap: number;
}

const VIEWER_LABEL = 'viewer';
const SHOW_EVENT = 'viewer-show';
const CONTEXT_EVENT = 'viewer-context';

/**
 * Whether this page runs in the viewer window: by its window label, or by the
 * `?window=viewer` query the window is opened with.
 */
export function isViewerWindow(): boolean {
  if (new URLSearchParams(window.location.search).get('window') === VIEWER_LABEL) return true;
  try {
    return getCurrentWindow().label === VIEWER_LABEL;
  } catch {
    // Outside Tauri (e.g. tests) there is no window label.
    return false;
  }
}

/** Shows a photo in the viewer window, opening the window if needed. */
export async function openViewer(request: ViewerRequest): Promise<void> {
  await invoke('open_viewer', { request });
}

/** The last photo request for the viewer, which a freshly loaded viewer window shows; `null` if none. */
export async function currentViewerRequest(): Promise<ViewerRequest | null> {
  return invoke<ViewerRequest | null>('current_viewer_request');
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
