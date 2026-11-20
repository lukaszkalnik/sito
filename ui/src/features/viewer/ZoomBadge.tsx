import { useTranslation } from 'react-i18next';

import { useFormats } from '../../i18n/formats';
import '../../controls/controls.css';

/** The zoom level of a zoomed photo (`scale` 1 = 100 %), with „Dopasuj” going back to fit. */
export function ZoomBadge({ scale, onFit }: { scale: number; onFit: () => void }) {
  const { t } = useTranslation();
  const formats = useFormats();
  return (
    <div className="viewer-zoom" role="group" aria-label={t('viewer.zoomLevel')}>
      <span>{formats.percent(scale)}</span>
      <button type="button" className="control-button" onClick={onFit}>
        {t('viewer.zoomFit')}
      </button>
    </div>
  );
}
