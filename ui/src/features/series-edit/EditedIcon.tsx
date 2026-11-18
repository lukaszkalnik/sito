import { useTranslation } from 'react-i18next';

/** Small pencil after the header of a series edited by hand. */
export function EditedIcon() {
  const { t } = useTranslation();
  return (
    <svg
      className="series-edited-icon"
      role="img"
      aria-label={t('seriesEdit.edited')}
      viewBox="0 0 24 24"
      width="13"
      height="13"
    >
      <title>{t('seriesEdit.edited')}</title>
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
