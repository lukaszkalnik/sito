import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { UpdateInfo } from './api';
import { ReleaseNotesDialog } from './ReleaseNotesDialog';
import { isShown, type UpdateState } from './updateReducer';
import { useUpdate } from './useUpdate';
import './UpdateNotice.css';

const PERCENT = new Intl.NumberFormat('pl-PL', { style: 'percent', maximumFractionDigits: 0 });

interface Actions {
  readonly install: () => void;
  readonly dismiss: () => void;
  readonly showNotes: () => void;
}

function Offer({
  info,
  installError,
  actions,
}: {
  info: UpdateInfo;
  installError: string | null;
  actions: Actions;
}) {
  const { t } = useTranslation();
  return (
    <>
      <p>{t('update.available', { version: info.version })}</p>
      {installError !== null && (
        <p role="alert">
          {t('update.installFailed', { reason: t(`update.errors.${installError}`) })}
        </p>
      )}
      {info.install === 'manual' && <p>{t('update.manual')}</p>}
      <div className="update-notice-buttons">
        {info.install === 'inApp' && (
          <button type="button" onClick={actions.install}>
            {t('update.install')}
          </button>
        )}
        {info.notes !== null && (
          <button type="button" onClick={actions.showNotes}>
            {t('update.whatsNew')}
          </button>
        )}
        <button type="button" onClick={actions.dismiss}>
          {t('update.notNow')}
        </button>
      </div>
    </>
  );
}

function Answer({ text, onClose }: { text: string; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <>
      <p>{text}</p>
      <div className="update-notice-buttons">
        <button type="button" onClick={onClose}>
          {t('update.close')}
        </button>
      </div>
    </>
  );
}

/** The content of the notice for `state`, or `null` when it shows nothing. */
function NoticeBody({ state, actions }: { state: UpdateState; actions: Actions }) {
  const { t } = useTranslation();
  switch (state.kind) {
    case 'idle':
      return null;
    case 'checking':
      return <p>{t('update.checking')}</p>;
    case 'available':
      return <Offer info={state.info} installError={state.installError} actions={actions} />;
    case 'upToDate':
      return (
        <Answer text={t('update.upToDate', { version: state.version })} onClose={actions.dismiss} />
      );
    case 'failed':
      return (
        <Answer
          text={t('update.checkFailed', { reason: t(`update.errors.${state.code}`) })}
          onClose={actions.dismiss}
        />
      );
    case 'downloading':
      return (
        <p>
          {state.fraction === null
            ? t('update.downloading')
            : t('update.downloadingPercent', { percent: PERCENT.format(state.fraction) })}
        </p>
      );
    case 'installing':
      return <p>{t('update.installing')}</p>;
  }
}

/**
 * A card at the bottom right of the main window that offers a newer version
 * of Sito and installs it, with its release notes in a dialog.
 */
export function UpdateNotice() {
  const { t } = useTranslation();
  const { state, install, dismiss } = useUpdate();
  const [notesOpen, setNotesOpen] = useState(false);
  const actions: Actions = { install, dismiss, showNotes: () => setNotesOpen(true) };
  const info = 'info' in state ? state.info : null;
  const body = <NoticeBody state={state} actions={actions} />;
  return (
    <>
      {/* Always rendered, so screen readers announce what appears in it. */}
      <section className="update-notice-area" aria-live="polite" aria-label={t('update.label')}>
        {isShown(state) && <div className="update-notice">{body}</div>}
      </section>
      <ReleaseNotesDialog
        open={notesOpen && info !== null}
        version={info?.version ?? ''}
        notes={info?.notes ?? ''}
        onClose={() => setNotesOpen(false)}
      />
    </>
  );
}
