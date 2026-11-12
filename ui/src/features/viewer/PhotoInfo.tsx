import { Fragment } from 'react';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

import { CutPath } from '../../paths/CutPath';
import { FolderIcon } from '../scan/FolderIcon';
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

/** Translation keys of a file size, per unit. */
const SIZE_KEYS: Readonly<Record<FileSize['unit'], string>> = {
  mb: 'viewer.sizeMb',
  kb: 'viewer.sizeKb',
};

const SEPARATOR = ' · ';

/** Date and time, resolution (when known; not for RAW) and file size of the photo. */
function factTexts(entry: TimelineEntry, details: PhotoDetails, t: TFunction): string[] {
  const size = fileSize(details.sizeBytes);
  const { resolution } = details;
  return [
    `${formatDay(dayOf(entry))}, ${timeOf(entry.takenAt)}`,
    resolution === null
      ? null
      : t('viewer.resolution', {
          width: formatPixels(resolution.width),
          height: formatPixels(resolution.height),
        }),
    t(SIZE_KEYS[size.unit], { size: size.value }),
  ].filter((text) => text !== null);
}

/** The second line: the facts and the date source note, joined with „·”; the whole line as tooltip. */
function PhotoFacts({ entry, details }: { entry: TimelineEntry; details: PhotoDetails }) {
  const { t } = useTranslation();
  const facts = factTexts(entry, details, t);
  const noteKey = DATE_SOURCE_NOTES[entry.timeSource];
  const note = noteKey === null ? null : t(noteKey);
  const line = [...facts, note].filter((text) => text !== null).join(SEPARATOR);
  return (
    <p className="viewer-facts" title={line}>
      {facts.map((text, position) => (
        <Fragment key={text}>
          {position > 0 && SEPARATOR}
          <span>{text}</span>
        </Fragment>
      ))}
      {note !== null && (
        <>
          {SEPARATOR}
          <span className="viewer-date-source">{note}</span>
        </>
      )}
    </p>
  );
}

function FormatNote({ format }: { format: PhotoFormat }) {
  const { t } = useTranslation();
  const badge = FORMAT_BADGES[format];
  return badge === null ? null : <span className="viewer-format">{t(badge)}</span>;
}

/** The photo's folder with a folder icon, cut in the middle; its tooltip names it a folder. */
function PhotoFolder({ relativeDir }: { relativeDir: string }) {
  const { t } = useTranslation();
  const path = relativeDir || t('timeline.rootFolder');
  return (
    <span className="viewer-folder">
      <FolderIcon size={14} />
      <span className="visually-hidden">{t('viewer.folderLabel')}</span>
      <CutPath path={path} title={t('viewer.folder', { path })} />
    </span>
  );
}

/**
 * The photo's info on two lines: name, format badge and folder; then date,
 * resolution, file size and date source note.
 */
export function PhotoInfo({ entry, details }: { entry: TimelineEntry; details: PhotoDetails }) {
  return (
    <div className="viewer-info">
      <p className="viewer-title">
        <span className="viewer-name" title={entry.name}>
          {entry.name}
        </span>
        <FormatNote format={entry.format} />
        <PhotoFolder relativeDir={entry.relativeDir} />
      </p>
      <PhotoFacts entry={entry} details={details} />
    </div>
  );
}
