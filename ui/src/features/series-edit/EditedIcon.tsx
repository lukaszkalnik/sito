import { useTranslation } from 'react-i18next';

/** The pencil of edited series and photos, drawn in the current text colour. */
function Pencil({ label }: { label: string }) {
  return (
    <svg
      className="series-edited-icon"
      role="img"
      aria-label={label}
      viewBox="0 0 24 24"
      width="13"
      height="13"
    >
      <title>{label}</title>
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
