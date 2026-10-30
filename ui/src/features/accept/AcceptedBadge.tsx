import { useTranslation } from 'react-i18next';

import { CheckIcon } from './CheckIcon';
import './AcceptedBadge.css';

/** Green check in the corner of a thumbnail of an accepted photo. */
export function AcceptedBadge() {
  const { t } = useTranslation();
  return (
    <span
      className="accepted-badge"
      role="img"
      aria-label={t('accept.badge')}
      title={t('accept.badge')}
    >
      <CheckIcon size={14} />
    </span>
  );
}
