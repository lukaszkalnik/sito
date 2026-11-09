import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { UndoButton } from './UndoButton';

function undoButton() {
  return screen.getByRole('button', { name: 'Cofnij' });
}

describe('UndoButton', () => {
  it('is the button „Cofnij” with its hint as tooltip', async () => {
    const onUndo = vi.fn();
    render(<UndoButton state="available" onUndo={onUndo} />);

    expect(undoButton()).toHaveAttribute('title', 'Przywróć ostatnio odrzucone zdjęcie (Ctrl+Z)');
    expect(undoButton()).toBeEnabled();
    expect(undoButton()).not.toHaveAttribute('aria-disabled');
    await userEvent.click(undoButton());
    expect(onUndo).toHaveBeenCalledOnce();
  });

  it('is disabled with nothing to undo', () => {
    render(<UndoButton state="none" onUndo={() => {}} />);

    expect(undoButton()).toBeDisabled();
  });

  it('looks disabled but stays clickable with only trash moves left', async () => {
    const onUndo = vi.fn();
    render(<UndoButton state="trashOnly" onUndo={onUndo} />);

    expect(undoButton()).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(undoButton());
    expect(onUndo).toHaveBeenCalledOnce();
  });
});
