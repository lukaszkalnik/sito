import { useTranslation } from 'react-i18next';

import type { PhotoFormat, TimeSource, TimelineEntry } from '../timeline/api';
import { FORMAT_BADGES } from '../timeline/formatBadges';
import { formatDay, timeOf } from '../timeline/format';
import { dayOf } from '../timeline/layout';
import type { PhotoDetails } from './api';
import { fileSize, formatPixels, type FileSize } from './photoDetails';
import './PhotoInfo.css';

/** Translation keys of the note on where a photo's date comes from; `null` for none. */
const DATE_SOURCE_NOTES: Readonly<Record<TimeSource, string | null>> = {
  exif: null,
  file: null,
  takeout: 'viewer.takeoutDate',
  takeoutCorrected: 'viewer.takeoutCorrected',
};

function DateSource({ source }: { source: TimeSource }) {
  const { t } = useTranslation();
  const note = DATE_SOURCE_NOTES[source];
  return note === null ? null : <span className="viewer-date-source">{t(note)}</span>;
}

function FormatNote({ format }: { format: PhotoFormat }) {
  const { t } = useTranslation();
  const badge = FORMAT_BADGES[format];
  return badge === null ? null : <span className="viewer-format">{t(badge)}</span>;
}

/** Translation keys of a file size, per unit. */
const SIZE_KEYS: Readonly<Record<FileSize['unit'], string>> = {
  mb: 'viewer.sizeMb',
  kb: 'viewer.sizeKb',
};

/** The photo's resolution (when known; not for RAW) and file size. */
function PhotoFacts({ details }: { details: PhotoDetails }) {
  const { t } = useTranslation();
  const size = fileSize(details.sizeBytes);
  const { resolution } = details;
  return (
    <>
      {resolution && (
        <span>
          {t('viewer.resolution', {
            width: formatPixels(resolution.width),
            height: formatPixels(resolution.height),
          })}
        </span>
      )}
      <span>{t(SIZE_KEYS[size.unit], { size: size.value })}</span>
    </>
  );
}

/** Name, date, details and folder of the photo. */
export function PhotoInfo({ entry, details }: { entry: TimelineEntry; details: PhotoDetails }) {
  const { t } = useTranslation();
  return (
    <p className="viewer-info">
      <span className="viewer-name">{entry.name}</span>
      <span>
        {formatDay(dayOf(entry))}, {timeOf(entry.takenAt)}
      </span>
      <PhotoFacts details={details} />
      <DateSource source={entry.timeSource} />
      <FormatNote format={entry.format} />
      <span className="viewer-folder">{entry.relativeDir || t('timeline.rootFolder')}</span>
    </p>
  );
}
