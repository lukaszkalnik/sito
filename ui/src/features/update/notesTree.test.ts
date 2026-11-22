import { describe, expect, it } from 'vitest';

import {
  parseReleaseNotes,
  releaseDay,
  type Block,
  type Inline,
  type NotesSection,
} from './notesTree';

const text = (value: string): Inline => ({ kind: 'text', text: value });
const code = (value: string): Inline => ({ kind: 'code', text: value });
const bold = (...content: Inline[]): Inline => ({ kind: 'bold', content });
const paragraph = (...content: Inline[]): Block => ({ kind: 'paragraph', content });
const heading = (value: string): Block => ({ kind: 'heading', content: [text(value)] });

/** The blocks of notes without a version heading. */
function blocksOf(notes: string): readonly Block[] {
  const sections = parseReleaseNotes(notes);
  expect(sections.map(({ version }) => version)).toEqual([null]);
  return sections[0]?.blocks ?? [];
}

/** The inline runs of a one-paragraph note. */
function inlineOf(notes: string): readonly Inline[] {
  const [block] = blocksOf(notes);
  return block?.kind === 'paragraph' ? block.content : [];
}

describe('parseReleaseNotes', () => {
  it('has no sections for empty or blank notes', () => {
    expect(parseReleaseNotes('')).toEqual([]);
    expect(parseReleaseNotes('  \n\n')).toEqual([]);
  });

  it('reads notes without a version heading as one section', () => {
    expect(parseReleaseNotes('### New\n- Updates')).toEqual<NotesSection[]>([
      {
        version: null,
        date: null,
        blocks: [heading('New'), { kind: 'list', items: [[text('Updates')]] }],
      },
    ]);
  });

  it('splits the notes at version headings, with and without a date', () => {
    const sections = parseReleaseNotes(
      '## v0.17.0 – 2026-12-01\n\n- Seventeen\n\n## v0.16.0\n\n- Sixteen\n## v0.15.1 - 2026-11-08\n',
    );
    expect(sections.map(({ version, date }) => [version, date])).toEqual([
      ['0.17.0', '2026-12-01'],
      ['0.16.0', null],
      ['0.15.1', '2026-11-08'],
    ]);
    expect(sections[1]?.blocks).toEqual([{ kind: 'list', items: [[text('Sixteen')]] }]);
    expect(sections[2]?.blocks).toEqual([]);
  });

  it('keeps text before the first version heading as its own section', () => {
    const sections = parseReleaseNotes('Intro\n## v0.16.0\n- Sixteen');
    expect(sections.map(({ version }) => version)).toEqual([null, '0.16.0']);
    expect(sections[0]?.blocks).toEqual([paragraph(text('Intro'))]);
  });

  it('reads one, two and many list items, ended by a blank line', () => {
    expect(blocksOf('- One')).toEqual([{ kind: 'list', items: [[text('One')]] }]);
    expect(blocksOf('- One\n- Two\n- Three\n\n- Four')).toEqual([
      { kind: 'list', items: [[text('One')], [text('Two')], [text('Three')]] },
      { kind: 'list', items: [[text('Four')]] },
    ]);
  });

  it('continues a list item on an indented or a lazy line', () => {
    expect(blocksOf('- One\n  more\nstill\n- Two')).toEqual([
      { kind: 'list', items: [[text('One more still')], [text('Two')]] },
    ]);
  });

  it('joins the lines of a paragraph and ends it at a blank line, a heading or a list', () => {
    expect(blocksOf('First\nline\n\nSecond\n### Heading\nThird\n- Item')).toEqual([
      paragraph(text('First line')),
      paragraph(text('Second')),
      heading('Heading'),
      paragraph(text('Third')),
      { kind: 'list', items: [[text('Item')]] },
    ]);
  });

  it('keeps unknown headings and other lines as text', () => {
    expect(blocksOf('# Title\n\n#### Small\n\n## Unreleased\n\n* star')).toEqual([
      paragraph(text('# Title')),
      paragraph(text('#### Small')),
      paragraph(text('## Unreleased')),
      paragraph(text('* star')),
    ]);
  });

  it('accepts Windows line endings', () => {
    const sections = parseReleaseNotes('## v0.16.0 – 2026-11-25\r\n\r\n### Fixed\r\n- One\r\n');
    expect(sections).toEqual<NotesSection[]>([
      {
        version: '0.16.0',
        date: '2026-11-25',
        blocks: [heading('Fixed'), { kind: 'list', items: [[text('One')]] }],
      },
    ]);
  });

  it('reads bold text and code, also code inside bold', () => {
    expect(inlineOf('A **bold** and `code` b')).toEqual([
      text('A '),
      bold(text('bold')),
      text(' and '),
      code('code'),
      text(' b'),
    ]);
    expect(inlineOf('**Files like `._x` are skipped.** Text')).toEqual([
      bold(text('Files like '), code('._x'), text(' are skipped.')),
      text(' Text'),
    ]);
  });

  it('takes code literally', () => {
    expect(inlineOf('`**not bold** [a](b)`')).toEqual([code('**not bold** [a](b)')]);
  });

  it('keeps markers without their closing one as text', () => {
    expect(inlineOf('An **open bold')).toEqual([text('An **open bold')]);
    expect(inlineOf('An `open code')).toEqual([text('An `open code')]);
    expect(inlineOf('Empty **** and `` markers')).toEqual([text('Empty **** and `` markers')]);
    expect(inlineOf('a * b [c] (d) [e](f')).toEqual([text('a * b [c] (d) [e](f')]);
  });

  it('shows a link as its text, without its address, also inside bold', () => {
    const notes =
      'Fixed ([slice 32](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/32-accept-photo.md), [32a](https://example.com/x))';
    expect(inlineOf(notes)).toEqual([text('Fixed (slice 32, 32a)')]);
    expect(inlineOf('**See [the page](https://example.com)**')).toEqual([
      bold(text('See the page')),
    ]);
    expect(JSON.stringify(parseReleaseNotes(notes))).not.toContain('http');
  });

  it('reads a real CHANGELOG section', () => {
    const notes = [
      '## v0.15.1 – 2026-11-08',
      '',
      '### Fixed',
      '',
      '- **Hidden macOS files are no longer shown as photos.** Sito skips the hidden `._` files. ([slice 38](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/38-files-changed-outside.md))',
    ].join('\n');
    expect(parseReleaseNotes(notes)).toEqual<NotesSection[]>([
      {
        version: '0.15.1',
        date: '2026-11-08',
        blocks: [
          heading('Fixed'),
          {
            kind: 'list',
            items: [
              [
                bold(text('Hidden macOS files are no longer shown as photos.')),
                text(' Sito skips the hidden '),
                code('._'),
                text(' files. (slice 38)'),
              ],
            ],
          },
        ],
      },
    ]);
  });
});

describe('releaseDay', () => {
  const section = (date: string | null): NotesSection => ({ version: '0.16.0', date, blocks: [] });

  it('is the local day of the heading', () => {
    expect(releaseDay(section('2026-11-25'))).toEqual(new Date(2026, 10, 25));
  });

  it('is null without a date or for an impossible day', () => {
    expect(releaseDay(section(null))).toBeNull();
    expect(releaseDay(section('2026-02-30'))).toBeNull();
    expect(releaseDay(section('2026-13-01'))).toBeNull();
  });
});
