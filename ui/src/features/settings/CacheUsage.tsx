import { useTranslation } from 'react-i18next';

import type { SizeState } from './useCacheSettings';
import { useSizeText } from './useSizeText';

/** How much space thumbnails and photo data take, or that it is being measured. */
export function CacheUsage({ size }: { size: SizeState }) {
  const { t } = useTranslation();
  const sizeText = useSizeText();
  switch (size.kind) {
    case 'measuring':
      return (
        <p role="status" className="cache-usage">
          <span className="cache-spinner" aria-hidden="true" />
          {t('cache.measuring')}
        </p>
      );
    case 'failed':
      return <p className="cache-usage">{t('cache.measureFailed')}</p>;
    case 'measured':
      return (
        <p className="cache-usage">
          {t('cache.usage', {
            thumbnails: sizeText(size.size.thumbnails),
            photoData: sizeText(size.size.photoData),
          })}
        </p>
      );
  }
}
