import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { BackgroundWork } from './backgroundWork';
import { CacheClearing } from './CacheClearing';
import { CacheLimitInput } from './CacheLimitInput';
import { CacheLocation } from './CacheLocation';
import { CacheUsage } from './CacheUsage';
import type { useCacheClearing } from './useCacheClearing';
import type { useCacheMove } from './useCacheMove';
import type { SettingsState, SizeState } from './useCacheSettings';
import './CacheSettingsDialog.css';

/** Everything the dialog shows and does. */
export interface CacheDialogProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly settings: SettingsState;
  readonly size: SizeState;
  readonly work: BackgroundWork;
  readonly onLimit: (mb: number) => void;
  readonly clearing: ReturnType<typeof useCacheClearing>;
  readonly move: ReturnType<typeof useCacheMove>;
}

function DialogBody(props: CacheDialogProps) {
  const { t } = useTranslation();
  const { settings, clearing } = props;
  switch (settings.kind) {
    case 'loading':
      return <p role="status">{t('cache.loading')}</p>;
    case 'failed':
      return <p role="alert">{t('cache.errors.internal')}</p>;
    case 'loaded':
      return (
        <>
          <section className="cache-section">
            <CacheLimitInput
              limitMb={settings.settings.thumbnailLimitMb}
              onChange={props.onLimit}
            />
            <CacheUsage size={props.size} />
            <CacheClearing
              state={clearing.state}
              photoDataLocked={props.work !== 'none'}
              onAsk={clearing.ask}
              onConfirm={clearing.confirm}
              onCancel={clearing.cancel}
            />
          </section>
          <CacheLocation
            settings={settings.settings}
            move={props.move}
            locked={props.work === 'scanning'}
          />
        </>
      );
  }
}

/** Modal dialog with the cache settings: size, emptying and the folder. */
export function CacheSettingsDialog(props: CacheDialogProps) {
  const { t } = useTranslation();
  const dialog = useRef<HTMLDialogElement>(null);
  const { open } = props;
  const moving = props.move.state.kind === 'moving';

  // The native modal traps focus and closes on Escape.
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);

  return (
    <dialog
      ref={dialog}
      className="cache-settings"
      aria-labelledby="cache-settings-title"
      onClose={props.onClose}
      onCancel={(event) => {
        // A move runs to its end or until stopped with „Anuluj”.
        if (moving) event.preventDefault();
      }}
    >
      <h2 id="cache-settings-title">{t('cache.title')}</h2>
      <DialogBody {...props} />
      <button type="button" className="cache-close" disabled={moving} onClick={props.onClose}>
        {t('cache.close')}
      </button>
    </dialog>
  );
}
