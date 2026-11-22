/**
 * The release notes as a typed tree: the small Markdown subset that
 * `CHANGELOG.md` uses (version and `###` headings, `-` lists, paragraphs,
 * `**bold**` and `` `code` ``). Links are reduced to the text they show, so
 * the tree holds no addresses; anything else stays plain text.
 */

/** A run of text inside a heading, a list item or a paragraph. */
export type Inline =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'bold'; readonly content: readonly Inline[] }
  | { readonly kind: 'code'; readonly text: string };

/** A block of a section. */
export type Block =
  | { readonly kind: 'heading'; readonly content: readonly Inline[] }
  | { readonly kind: 'list'; readonly items: readonly (readonly Inline[])[] }
  | { readonly kind: 'paragraph'; readonly content: readonly Inline[] };

/** The notes of one version, or the notes without a version heading. */
export interface NotesSection {
  /** The version without its `v`, e.g. `0.16.0`; `null` without a heading. */
  readonly version: string | null;
  /** The release day as `YYYY-MM-DD`, if the heading gives one. */
  readonly date: string | null;
  readonly blocks: readonly Block[];
}

/** A block while its lines are being read. */
type Draft =
  | { kind: 'heading'; text: string }
  | { kind: 'list'; items: string[] }
  | { kind: 'paragraph'; lines: string[] };

/** `## v0.16.0 – 2026-11-25`, the date optional. */
const VERSION_HEADING = /^## v(\d+\.\d+\.\d+\S*)(?:\s+[–-]\s+(\d{4}-\d{2}-\d{2}))?\s*$/u;
const SUB_HEADING = /^### (.*)$/u;
const LIST_ITEM = /^- (.*)$/u;
/** `[text](address)`, with no spaces in the address. */
const LINK = /^\[([^\]]*)\]\([^)\s]*\)/u;

/** Parses `notes` into its sections, newest first as written. */
export function parseReleaseNotes(notes: string): readonly NotesSection[] {
  const sections: { version: string | null; date: string | null; lines: string[] }[] = [];
  for (const line of notes.split(/\r?\n/u)) {
    const heading = VERSION_HEADING.exec(line);
    const last = sections.at(-1);
    if (heading) {
      sections.push({ version: heading[1] ?? null, date: heading[2] ?? null, lines: [] });
    } else if (last) {
      last.lines.push(line);
    } else {
      sections.push({ version: null, date: null, lines: [line] });
    }
  }
  return sections
    .map(({ version, date, lines }) => ({ version, date, blocks: parseBlocks(lines) }))
    .filter((section) => section.version !== null || section.blocks.length > 0);
}

/** The blocks of the lines of one section. */
function parseBlocks(lines: readonly string[]): readonly Block[] {
  const drafts: Draft[] = [];
  // The block the next line continues, if any: a blank line or a heading ends it.
  let open: Draft | null = null;
  for (const line of lines) {
    open = addLine(drafts, open, line);
  }
  return drafts.map(toBlock);
}

/** Adds `line` to `drafts`, returning the block the next line continues. */
function addLine(drafts: Draft[], open: Draft | null, line: string): Draft | null {
  if (line.trim() === '') return null;
  const heading = SUB_HEADING.exec(line);
  if (heading) {
    drafts.push({ kind: 'heading', text: heading[1] ?? '' });
    return null;
  }
  const item = LIST_ITEM.exec(line);
  if (item) {
    const list = open?.kind === 'list' ? open : startBlock(drafts, { kind: 'list', items: [] });
    list.items.push(item[1] ?? '');
    return list;
  }
  return continueBlock(drafts, open, line.trim());
}

/** Appends a line that is neither a heading nor a list item to `open`. */
function continueBlock(drafts: Draft[], open: Draft | null, text: string): Draft {
  if (open?.kind === 'list') {
    // An indented line (or a lazy one) continues the list's last item.
    const last = open.items.length - 1;
    open.items[last] = `${open.items[last] ?? ''} ${text}`;
    return open;
  }
  if (open?.kind === 'paragraph') {
    open.lines.push(text);
    return open;
  }
  return startBlock(drafts, { kind: 'paragraph', lines: [text] });
}

function startBlock<T extends Draft>(drafts: Draft[], draft: T): T {
  drafts.push(draft);
  return draft;
}

function toBlock(draft: Draft): Block {
  switch (draft.kind) {
    case 'heading':
      return { kind: 'heading', content: parseInline(draft.text) };
    case 'list':
      return { kind: 'list', items: draft.items.map(parseInline) };
    case 'paragraph':
      return { kind: 'paragraph', content: parseInline(draft.lines.join(' ')) };
  }
}

/** The inline runs of `text`. A marker without its closing one stays text. */
function parseInline(text: string): readonly Inline[] {
  const runs: Inline[] = [];
  let rest = text;
  while (rest !== '') {
    const [run, length] = nextRun(rest);
    appendRun(runs, run);
    rest = rest.slice(length);
  }
  return runs;
}

/** The run at the start of `text` and how many characters it takes. */
function nextRun(text: string): readonly [Inline, number] {
  if (text.startsWith('`')) {
    const end = text.indexOf('`', 1);
    if (end > 1) return [{ kind: 'code', text: text.slice(1, end) }, end + 1];
  }
  if (text.startsWith('**')) {
    const end = text.indexOf('**', 2);
    if (end > 2) return [{ kind: 'bold', content: parseInline(text.slice(2, end)) }, end + 2];
  }
  const link = LINK.exec(text);
  if (link) return [{ kind: 'text', text: link[1] ?? '' }, link[0].length];
  // Plain text up to the next character that may start a marker.
  const next = text.slice(1).search(/[`*[]/u);
  const length = next < 0 ? text.length : next + 1;
  return [{ kind: 'text', text: text.slice(0, length) }, length];
}

/** Appends `run`, merging neighbouring plain text into one run. */
function appendRun(runs: Inline[], run: Inline): void {
  const last = runs.at(-1);
  if (run.kind === 'text' && last?.kind === 'text') {
    runs[runs.length - 1] = { kind: 'text', text: last.text + run.text };
  } else if (run.kind !== 'text' || run.text !== '') {
    runs.push(run);
  }
}

/** The release day of `section` as a local date, if it has a valid one. */
export function releaseDay(section: NotesSection): Date | null {
  const [year, month, day] = (section.date ?? '').split('-').map(Number);
  if (year === undefined || month === undefined || day === undefined) return null;
  const date = new Date(year, month - 1, day);
  // `Date` rolls an impossible day like 2026-02-30 over into the next month.
  const valid =
    date.getFullYear() === year && date.getMonth() === month - 1 && date.getDate() === day;
  return valid ? date : null;
}
