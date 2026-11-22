import type { BackgroundWork } from '../settings/backgroundWork';
import type { ScanState } from './scanReducer';

/** What the background work of `state` is doing, for the cache settings. */
export function backgroundWork(state: ScanState): BackgroundWork {
  const shown = state.status === 'picking' ? state.previous : state;
  if (shown.status === 'scanning') return 'scanning';
  if (shown.status !== 'done') return 'none';
  if (shown.summary.refreshing) return 'scanning';
  return shown.completing === null ? 'none' : 'exifPass';
}
