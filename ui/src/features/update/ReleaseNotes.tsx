import type { ReactElement } from 'react';
import { useTranslation } from 'react-i18next';

import { useFormats } from '../../i18n/formats';
import {
  parseReleaseNotes,
  releaseDay,
  type Block,
  type Inline,
  type NotesSection,
} from './notesTree';

// The notes are parsed once per render and never reorder, so the position of
// a block or a run is a stable key for it.

/**
 * The release notes, formatted: a heading per version, then its headings,
 * lists and paragraphs. Built from React elements only, never from HTML, and
 * without links.
 */
export function ReleaseNotes({ notes }: { notes: string }) {
  const sections = parseReleaseNotes(notes);
  return (
    <>
      {sections.map((section, index) => (
        // oxlint-disable-next-line react/no-array-index-key -- static notes, see above
        <Section key={index} section={section} />
      ))}
    </>
  );
}

function Section({ section }: { section: NotesSection }) {
  const nested = section.version !== null;
  return (
    <section className="release-notes-section">
      {nested && <VersionHeading section={section} />}
      {section.blocks.map((block, index) => (
        // oxlint-disable-next-line react/no-array-index-key -- static notes, see above
        <NotesBlock key={index} block={block} nested={nested} />
      ))}
    </section>
  );
}

function VersionHeading({ section }: { section: NotesSection }) {
  const { t } = useTranslation();
  const formats = useFormats();
  const day = releaseDay(section);
  const version = section.version ?? '';
  return (
    <h3>
      {day === null
        ? t('update.notesVersion', { version })
        : t('update.notesVersionDated', { version, date: formats.day(day) })}
    </h3>
  );
}

/** A block; its headings are a level lower under a version heading. */
function NotesBlock({ block, nested }: { block: Block; nested: boolean }): ReactElement {
  switch (block.kind) {
    case 'heading':
      return nested ? <h4>{runs(block.content)}</h4> : <h3>{runs(block.content)}</h3>;
    case 'list':
      return (
        <ul>
          {block.items.map((item, index) => (
            // oxlint-disable-next-line react/no-array-index-key -- static notes, see above
            <li key={index}>{runs(item)}</li>
          ))}
        </ul>
      );
    case 'paragraph':
      return <p>{runs(block.content)}</p>;
  }
}

function runs(content: readonly Inline[]) {
  return content.map((run, index) => (
    // oxlint-disable-next-line react/no-array-index-key -- static notes, see above
    <Run key={index} run={run} />
  ));
}

function Run({ run }: { run: Inline }): ReactElement | string {
  switch (run.kind) {
    case 'text':
      return run.text;
    case 'bold':
      return <strong>{runs(run.content)}</strong>;
    case 'code':
      return <code>{run.text}</code>;
  }
}
