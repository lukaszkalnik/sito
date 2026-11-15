import { render } from '@testing-library/react';
import { vi } from 'vitest';

import type { UndoState } from '../reject/api';
import type { TimelineEntry } from '../timeline/api';
import type { ViewerPhoto } from './api';
import { PhotoViewer } from './PhotoViewer';

/** Photo `id` of six, one second apart, newest first. */
export function entry(id: number, timeSource: TimelineEntry['timeSource'] = 'exif'): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: id === 0 ? '' : 'Camera',
    relativePath: `IMG_${id}.jpg`,
    takenAt: `2024-06-01T12:00:0${9 - id}`,
    timeSource,
    format: 'jpeg',
  };
}

/**
 * The page of photo `id` in the all view of six photos, where photos 1–3
 * form a series and 0, 4 and 5 are single photos.
 */
export function pageOf(id: number, changes: Partial<ViewerPhoto> = {}): ViewerPhoto {
  const inSeries = id >= 1 && id <= 3;
  return {
    entry: entry(id),
    previous: id > 0 ? id - 1 : null,
    next: id < 5 ? id + 1 : null,
    series: inSeries ? [1, 2, 3].map((member) => entry(member)) : [],
    afterReject: null,
    details: { sizeBytes: 7_835_038, resolution: { width: 5472, height: 3648 } },
    ...changes,
  };
}

export function viewerProps(
  photo: ViewerPhoto,
  undoState: UndoState = 'none',
  acceptedIds: ReadonlySet<number> = new Set(),
) {
  return {
    photo,
    scanId: 3,
    onShow: vi.fn(),
    onClose: vi.fn(),
    onReject: vi.fn(),
    rejectTo: 'folder' as const,
    acceptedIds,
    onToggleAccept: vi.fn(),
    undoState,
    onUndo: vi.fn(),
    onRefresh: vi.fn(),
  };
}

export function renderViewer(photo: ViewerPhoto, undoState: UndoState = 'none') {
  const props = viewerProps(photo, undoState);
  render(<PhotoViewer {...props} />);
  return props;
}
