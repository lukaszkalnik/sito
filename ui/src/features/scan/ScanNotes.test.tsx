import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import '../../i18n';
import { ScanNotes } from './ScanNotes';

describe('ScanNotes', () => {
  it('shows nothing without background work', () => {
    render(<ScanNotes refreshing={false} completing={null} />);

    expect(screen.queryByText(/Sprawdzanie zmian|Uzupełnianie dat/)).not.toBeInTheDocument();
  });

  it('shows the background refresh', () => {
    render(<ScanNotes refreshing completing={null} />);

    expect(screen.getByText('Sprawdzanie zmian…')).toBeInTheDocument();
  });

  it('shows how far reading EXIF has got, with Polish number formatting', () => {
    render(<ScanNotes refreshing={false} completing={{ done: 1234, total: 34600 }} />);

    // pl-PL groups digits with a space from five digits on.
    expect(screen.getByText(/^Uzupełnianie dat z EXIF: 1234 z 34\s600\.$/)).toBeInTheDocument();
  });

  it('shows both notes together', () => {
    render(<ScanNotes refreshing completing={{ done: 5, total: 10 }} />);

    expect(
      screen.getByText('Sprawdzanie zmian… Uzupełnianie dat z EXIF: 5 z 10.'),
    ).toBeInTheDocument();
  });
});
