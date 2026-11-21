import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { ReleaseNotesDialog } from './ReleaseNotesDialog';
import '../../i18n';

const TWO_VERSIONS = [
  '## v0.17.0 – 2026-12-01',
  '',
  '### New',
  '',
  '- **Formatted notes.** Shows `code` too. ([slice 54](https://github.com/lukaszkalnik/sito/blob/main/docs/slices/54-release-notes-markdown.md))',
  '- Second item',
  '',
  '## v0.16.0',
  '',
  '### Fixed',
  '',
  '- A fix',
].join('\n');

function renderDialog(notes: string, onClose = () => {}) {
  render(<ReleaseNotesDialog open version="0.17.0" notes={notes} onClose={onClose} />);
  return screen.getByRole('dialog', { name: 'Co nowego w wersji 0.17.0' });
}

describe('ReleaseNotesDialog', () => {
  it('shows each version with its headings and lists', () => {
    const dialog = renderDialog(TWO_VERSIONS);

    expect(
      within(dialog)
        .getAllByRole('heading')
        .map((heading) => heading.textContent),
    ).toEqual([
      'Co nowego w wersji 0.17.0',
      'Wersja 0.17.0 · 1 grudnia 2026',
      'New',
      'Wersja 0.16.0',
      'Fixed',
    ]);
    const lists = within(dialog).getAllByRole('list');
    expect(within(lists[0] as HTMLElement).getAllByRole('listitem')).toHaveLength(2);
    expect(within(lists[1] as HTMLElement).getByRole('listitem')).toHaveTextContent('A fix');
  });

  it('formats bold text and code, and shows a link as its text only', () => {
    const dialog = renderDialog(TWO_VERSIONS);

    expect(within(dialog).getByText('Formatted notes.').tagName).toBe('STRONG');
    expect(within(dialog).getByText('code').tagName).toBe('CODE');
    expect(within(dialog).queryByRole('link')).not.toBeInTheDocument();
    expect(dialog).toHaveTextContent('Shows code too. (slice 54)');
    expect(dialog.innerHTML).not.toContain('github.com');
    expect(dialog).not.toHaveTextContent('**');
  });

  it('shows notes without version headings without a version heading', () => {
    const dialog = renderDialog('### New\n- Updates in the app');

    expect(within(dialog).getByRole('heading', { level: 3 })).toHaveTextContent('New');
    expect(within(dialog).getByRole('listitem')).toHaveTextContent('Updates in the app');
  });

  it('keeps the notes scrollable by keyboard and closes with „Zamknij”', async () => {
    const onClose = vi.fn();
    const dialog = renderDialog(TWO_VERSIONS, onClose);

    const notes = within(dialog).getByRole('region', { name: 'Opis zmian (po angielsku)' });
    expect(notes).toHaveAttribute('tabindex', '0');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Zamknij' }));
    expect(onClose).toHaveBeenCalled();
  });
});
