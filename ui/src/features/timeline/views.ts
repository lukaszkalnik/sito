import type { TimelineMode } from './layout';

/** Translation key of each view's name, in the order the „Widok” dropdown lists them. */
export const VIEW_LABELS: Readonly<Record<TimelineMode, 'view.series' | 'view.all'>> = {
  series: 'view.series',
  all: 'view.all',
};

/** The views in display order. */
export const VIEWS: readonly TimelineMode[] = Object.keys(VIEW_LABELS).filter(isView);

function isView(value: string): value is TimelineMode {
  return Object.hasOwn(VIEW_LABELS, value);
}

/** The view a dropdown value names, `null` for an unknown value. */
export function parseView(value: string): TimelineMode | null {
  return isView(value) ? value : null;
}
