import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { CutPath } from '../../paths/CutPath';
import { revealCacheFolder, type CachePlatform, type CacheSettings } from './api';
import { Confirmation } from './Confirmation';
import type { MoveState, MoveTarget } from './moveReducer';
import type { useCacheMove } from './useCacheMove';
import { useSizeText } from './useSizeText';

type Move = ReturnType<typeof useCacheMove>;

/** „Pokaż w Finderze” / „Pokaż w menedżerze plików”: where moving is not offered. */
const REVEAL_LABEL: Readonly<Record<Exclude<CachePlatform, 'windows'>, string>> = {
  macos: 'cache.revealFinder',
  other: 'cache.revealFileManager',
};

/** Shows the cache folder in the file manager, saying so if that fails. */
function RevealButton({ platform }: { platform: Exclude<CachePlatform, 'windows'> }) {
  const { t } = useTranslation();
  const [failed, setFailed] = useState(false);
  return (
    <>
      <div className="cache-buttons">
        <button
          type="button"
          onClick={() => {
            setFailed(false);
            revealCacheFolder().catch(() => setFailed(true));
          }}
        >
          {t(REVEAL_LABEL[platform])}
        </button>
      </div>
      {failed && <p role="alert">{t('cache.errors.revealFailed')}</p>}
    </>
  );
}

/** „Zmień…” and, away from the default folder, „Przywróć domyślny” (Windows). */
function MoveButtons({
  settings,
  move,
  locked,
}: {
  settings: CacheSettings;
  move: Move;
  locked: boolean;
}) {
  const { t } = useTranslation();
  const settled = move.state.kind === 'idle' || move.state.kind === 'failed';
  const disabled = locked || !settled;
  // In the fallback the default folder is in use already: nothing to move.
  const restore = settings.location.kind === 'fallback' ? move.useDefault : move.restoreDefault;
  return (
    <div className="cache-buttons">
      <button type="button" disabled={disabled} onClick={move.choose}>
        {t('cache.change')}
      </button>
      {settings.location.kind !== 'default' && (
        <button type="button" disabled={disabled} onClick={restore}>
          {t('cache.restoreDefault')}
        </button>
      )}
    </div>
  );
}

/** The confirmation of moving to `target`. */
function MoveConfirmation({ target, move }: { target: MoveTarget; move: Move }) {
  const { t } = useTranslation();
  const sizeText = useSizeText();
  const question =
    target.kind === 'chosen'
      ? t('cache.moveConfirm', {
          size: sizeText(target.checked.size),
          folder: target.checked.target,
        })
      : t('cache.restoreConfirm');
  return (
    <Confirmation
      question={question}
      confirmLabel={t('cache.move')}
      onConfirm={move.confirm}
      onCancel={move.dismiss}
    >
      {target.kind === 'chosen' && target.checked.network && (
        <p className="cache-warning">{t('cache.networkWarning')}</p>
      )}
    </Confirmation>
  );
}

/** What the move is doing now: its question, its progress or how it failed. */
function MoveStatus({ state, move }: { state: MoveState; move: Move }) {
  const { t } = useTranslation();
  switch (state.kind) {
    case 'idle':
    case 'choosing':
      return null;
    case 'confirming':
      return <MoveConfirmation target={state.target} move={move} />;
    case 'moving':
      return (
        <div className="cache-progress">
          <p role="status">
            <span className="cache-spinner" aria-hidden="true" />
            {state.progress === null
              ? t('cache.movingStart')
              : t('cache.moving', { done: state.progress.done, count: state.progress.total })}
          </p>
          <button type="button" onClick={move.stop}>
            {t('cache.cancel')}
          </button>
        </div>
      );
    case 'failed':
      return state.code === 'cancelled' ? (
        <p role="status">{t('cache.errors.cancelled')}</p>
      ) : (
        <p role="alert">{t(`cache.errors.${state.code}`)}</p>
      );
  }
}

/** The cache folder: where it is and, on Windows, moving it. */
export function CacheLocation({
  settings,
  move,
  locked,
}: {
  settings: CacheSettings;
  move: Move;
  /** A folder is being read: the cache cannot move now. */
  locked: boolean;
}) {
  const { t } = useTranslation();
  return (
    <section className="cache-section" aria-labelledby="cache-folder-title">
      <p id="cache-folder-title" className="cache-caption">
        {t('cache.folder')}
      </p>
      <CutPath path={settings.folder} className="cache-folder" />
      {settings.platform === 'windows' ? (
        <>
          <MoveButtons settings={settings} move={move} locked={locked} />
          {locked && <p className="cache-hint">{t('cache.busyHint')}</p>}
          <MoveStatus state={move.state} move={move} />
        </>
      ) : (
        <RevealButton platform={settings.platform} />
      )}
    </section>
  );
}
