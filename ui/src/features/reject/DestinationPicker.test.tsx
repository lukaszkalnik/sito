import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import '../../i18n';
import { loadDestination } from './destination';
import { DestinationSetting } from './DestinationPicker';

afterEach(() => localStorage.clear());

function picker() {
  return screen.getByRole('combobox', { name: 'Odrzucone zdjęcia do:' });
}

async function chooseTrash() {
  render(<DestinationSetting />);
  await userEvent.selectOptions(picker(), 'kosza');
}

describe('DestinationPicker', () => {
  it('starts with the folder', () => {
    render(<DestinationSetting />);

    expect(picker()).toHaveValue('folder');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('asks before switching to the trash', async () => {
    await chooseTrash();

    expect(
      screen.getByRole('dialog', { name: 'Przenosić odrzucone zdjęcia do kosza?' }),
    ).toBeInTheDocument();
    expect(picker()).toHaveValue('folder');
  });

  it('keeps the folder when the question is cancelled', async () => {
    await chooseTrash();
    await userEvent.click(screen.getByRole('button', { name: 'Anuluj' }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(picker()).toHaveValue('folder');
    expect(loadDestination()).toBe('folder');
  });

  it('switches to the trash, remembered and emphasised, once confirmed', async () => {
    await chooseTrash();
    await userEvent.click(screen.getByRole('button', { name: 'Tak, do kosza' }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(picker()).toHaveValue('trash');
    expect(loadDestination()).toBe('trash');
    expect(picker().closest('label')).toHaveClass('reject-destination--trash');
  });

  it('switches back to the folder without asking', async () => {
    localStorage.setItem('sito.rejectDestination', 'trash');
    render(<DestinationSetting />);

    await userEvent.selectOptions(picker(), 'folderu „Odrzucone”');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(picker()).toHaveValue('folder');
    expect(loadDestination()).toBe('folder');
  });
});
