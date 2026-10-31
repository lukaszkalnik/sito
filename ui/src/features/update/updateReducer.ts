import type { InstallProgress, UpdateErrorCode, UpdateInfo } from './api';

/** What started a check: the app's start (quiet) or the menu (always answers). */
export type CheckTrigger = 'start' | 'menu';

/** The update notice. `idle` shows nothing. */
export type UpdateState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'checking'; readonly trigger: CheckTrigger }
  | {
      readonly kind: 'available';
      readonly info: UpdateInfo;
      /** Why the last install failed, or `null`. */
      readonly installError: UpdateErrorCode | null;
    }
  | { readonly kind: 'upToDate'; readonly version: string }
  | { readonly kind: 'failed'; readonly code: UpdateErrorCode }
  | { readonly kind: 'downloading'; readonly info: UpdateInfo; readonly fraction: number | null }
  | { readonly kind: 'installing'; readonly info: UpdateInfo };

export type UpdateEvent =
  | { readonly type: 'checkStarted'; readonly trigger: CheckTrigger }
  | { readonly type: 'found'; readonly info: UpdateInfo }
  | { readonly type: 'notFound'; readonly currentVersion: string }
  | { readonly type: 'checkFailed'; readonly code: UpdateErrorCode }
  | { readonly type: 'installStarted' }
  | { readonly type: 'progress'; readonly progress: InstallProgress }
  | { readonly type: 'installFailed'; readonly code: UpdateErrorCode }
  | { readonly type: 'dismissed' };

export const initialUpdateState: UpdateState = { kind: 'idle' };

/** Whether an install is under way, which no other event may interrupt. */
function isInstalling(
  state: UpdateState,
): state is Extract<UpdateState, { kind: 'downloading' | 'installing' }> {
  return state.kind === 'downloading' || state.kind === 'installing';
}

/**
 * The notice after `event`. A check started at the app's start stays quiet
 * when there is no update or it fails; one from the menu always answers.
 * A failed install goes back to the offer, with the reason.
 */
export function updateReducer(state: UpdateState, event: UpdateEvent): UpdateState {
  switch (event.type) {
    case 'checkStarted':
      return isInstalling(state) ? state : { kind: 'checking', trigger: event.trigger };
    case 'found':
      return state.kind === 'checking'
        ? { kind: 'available', info: event.info, installError: null }
        : state;
    case 'notFound':
      return answerOfCheck(state, { kind: 'upToDate', version: event.currentVersion });
    case 'checkFailed':
      return answerOfCheck(state, { kind: 'failed', code: event.code });
    case 'installStarted':
      return state.kind === 'available'
        ? { kind: 'downloading', info: state.info, fraction: null }
        : state;
    case 'progress':
      return isInstalling(state) ? progressed(state.info, event.progress) : state;
    case 'installFailed':
      return isInstalling(state)
        ? { kind: 'available', info: state.info, installError: event.code }
        : state;
    case 'dismissed':
      return isInstalling(state) ? state : initialUpdateState;
    default:
      return assertNever(event);
  }
}

/** Whether the notice shows anything: a check at the start is quiet until it finds an update. */
export function isShown(state: UpdateState): boolean {
  if (state.kind === 'checking') return state.trigger === 'menu';
  return state.kind !== 'idle';
}

/** `answer` for a check from the menu; nothing for one at the start. */
function answerOfCheck(state: UpdateState, answer: UpdateState): UpdateState {
  if (state.kind !== 'checking') return state;
  return state.trigger === 'menu' ? answer : initialUpdateState;
}

function progressed(info: UpdateInfo, progress: InstallProgress): UpdateState {
  switch (progress.kind) {
    case 'downloading':
      return {
        kind: 'downloading',
        info,
        fraction: fractionOf(progress.downloaded, progress.total),
      };
    case 'installing':
      return { kind: 'installing', info };
    default:
      return assertNever(progress);
  }
}

/** `downloaded` of `total` bytes as 0–1, or `null` if the size is unknown. */
function fractionOf(downloaded: number, total: number | null): number | null {
  if (total === null || total <= 0) return null;
  return Math.min(downloaded / total, 1);
}

function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${JSON.stringify(value)}`);
}
