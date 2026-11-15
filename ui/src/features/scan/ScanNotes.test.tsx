import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { ScanNotes } from './ScanNotes';

const noMissing = { photosMissing: false, onRefresh: () => {} };

describe('ScanNotes', () => {
  it('shows nothing without background work', () => {
    render(<ScanNotes {...noMissing} refreshing={false} completing={null} />);

    expect(screen.queryByText(/Sprawdzanie zmian|Uzupełnianie dat/)).not.toBeInTheDocument();
  });

  it('shows the background refresh', () => {
    render(<ScanNotes {...noMissing} refreshing completing={null} />);

    expect(screen.getByText('Sprawdzanie zmian…')).toBeInTheDocument();
  });

  it('shows how far reading EXIF has got, with Polish number formatting', () => {
    render(
      <ScanNotes {...noMissing} refreshing={false} completing={{ done: 1234, total: 34600 }} />,
    );

    // pl-PL groups digits with a space from five digits on.
    expect(screen.getByText(/^Uzupełnianie dat z EXIF: 1234 z 34\s600\.$/)).toBeInTheDocument();
  });

  it('shows both notes together', () => {
    render(<ScanNotes {...noMissing} refreshing completing={{ done: 5, total: 10 }} />);

    expect(
      screen.getByText('Sprawdzanie zmian… Uzupełnianie dat z EXIF: 5 z 10.'),
    ).toBeInTheDocument();
  });

  it('notes missing photos with a button to refresh', async () => {
    const onRefresh = vi.fn();
    render(<ScanNotes refreshing={false} completing={null} photosMissing onRefresh={onRefresh} />);

    expect(
      screen.getByText(
        'Niektórych zdjęć nie ma już w otwartym folderze – zostały przeniesione, zmieniły nazwę lub usunięto je.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Odśwież' }));
    expect(onRefresh).toHaveBeenCalledOnce();
  });
});
