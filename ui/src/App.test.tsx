import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import App from './App';
import './i18n';

vi.mock('./features/about/api', () => ({
  onShowLicenses: vi.fn(() => Promise.resolve(() => {})),
}));

describe('App', () => {
  it('renders the Polish title and folder button', () => {
    render(<App />);

    expect(screen.getByRole('heading', { name: 'Sito' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeInTheDocument();
  });
});
