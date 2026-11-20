import { useTranslation } from 'react-i18next';

/**
 * The pencil of edited series and photos, drawn in the current text colour:
 * an image named `label`, or decoration without one.
 */
function Pencil({ label, size = 13 }: { label?: string; size?: number }) {
  const named =
    label === undefined ? { 'aria-hidden': true } : { role: 'img', 'aria-label': label };
  return (
    <svg className="series-edited-icon" {...named} viewBox="0 0 24 24" width={size} height={size}>
      {label !== undefined && <title>{label}</title>}
      <path
        d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/** Small pencil after the header of a series edited by hand. */
export function EditedIcon() {
  const { t } = useTranslation();
  return <Pencil label={t('seriesEdit.edited')} />;
}

/** Pencil in the top-left corner of a photo taken out of its series by hand. */
export function PinnedBadge() {
  const { t } = useTranslation();
  return (
    <span className="series-pinned-badge" title={t('seriesEdit.pinned')}>
      <Pencil label={t('seriesEdit.pinned')} />
    </span>
  );
}

/** Small pencil after the name of a photo taken out of its series by hand. */
export function PinnedIcon() {
  const { t } = useTranslation();
  return <Pencil label={t('seriesEdit.pinned')} />;
}

/** The pencil of an edit button, as decoration (the button names itself). */
export function EditIcon() {
  return <Pencil size={16} />;
}
