import { useTranslation } from 'react-i18next';

const PERCENT = new Intl.NumberFormat('pl-PL', { style: 'percent', maximumFractionDigits: 0 });

/** The zoom level of a zoomed photo (`scale` 1 = 100 %), with „Dopasuj” going back to fit. */
export function ZoomBadge({ scale, onFit }: { scale: number; onFit: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="viewer-zoom" role="group" aria-label={t('viewer.zoomLevel')}>
      <span>{PERCENT.format(scale)}</span>
      <button type="button" onClick={onFit}>
        {t('viewer.zoomFit')}
      </button>
    </div>
  );
}
