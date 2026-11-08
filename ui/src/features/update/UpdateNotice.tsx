import { useState } from 'react';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import type { UpdateInfo } from './api';
import { ReleaseNotesDialog } from './ReleaseNotesDialog';
import { isShown, type FailedAction, type Failure, type UpdateState } from './updateReducer';
import { useUpdate } from './useUpdate';
import './UpdateNotice.css';

const PERCENT = new Intl.NumberFormat('pl-PL', { style: 'percent', maximumFractionDigits: 0 });

/** The message key of each failed action; `Record` makes a new action a type error here. */
const FAILURE_MESSAGE: Readonly<Record<FailedAction, string>> = {
  install: 'update.installFailed',
};

/** The message of `failure`: what failed and why. */
function failureText(failure: Failure, t: TFunction): string {
  return t(FAILURE_MESSAGE[failure.action], { reason: t(`update.errors.${failure.code}`) });
}

interface Actions {
  readonly install: () => void;
  readonly dismiss: () => void;
  readonly showNotes: () => void;
}

function Offer({
  info,
  failure,
  actions,
}: {
  info: UpdateInfo;
  failure: Failure | null;
  actions: Actions;
}) {
  const { t } = useTranslation();
  return (
    <>
      <p>{t('update.available', { version: info.version })}</p>
      {failure !== null && <p role="alert">{failureText(failure, t)}</p>}
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
      return <Offer info={state.info} failure={state.failure} actions={actions} />;
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
