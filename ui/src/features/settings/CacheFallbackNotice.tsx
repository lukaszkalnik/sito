import { useTranslation } from 'react-i18next';

import '../update/UpdateNotice.css';
import './CacheSettingsDialog.css';

/** Says that the chosen cache folder is missing and the default one is used. */
export function CacheFallbackNotice({
  chosen,
  onOpenSettings,
  onDismiss,
}: {
  chosen: string;
  onOpenSettings: () => void;
  onDismiss: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="update-notice-area cache-fallback-area">
      <div className="update-notice" role="status">
        <p>{t('cache.fallback', { folder: chosen })}</p>
        <div className="update-notice-buttons">
          <button type="button" onClick={onOpenSettings}>
            {t('cache.openSettings')}
          </button>
          <button type="button" onClick={onDismiss}>
            {t('cache.close')}
          </button>
        </div>
      </div>
    </div>
  );
}
