import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import App from './App';
import './i18n';

describe('App', () => {
  it('renders the Polish title and folder button', () => {
    render(<App />);

    expect(screen.getByRole('heading', { name: 'Sito' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeInTheDocument();
  });
});
