import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { onFirstClick } from './firstClick';

function renderButton(action: () => void) {
  render(
    <button type="button" onClick={onFirstClick(action)}>
      Odrzuć
    </button>,
  );
  return screen.getByRole('button', { name: 'Odrzuć' });
}

describe('onFirstClick', () => {
  it('acts once on a double-click', () => {
    const action = vi.fn();
    const button = renderButton(action);

    fireEvent.click(button, { detail: 1 });
    fireEvent.click(button, { detail: 2 });

    expect(action).toHaveBeenCalledOnce();
  });

  it('acts on every separate click and on keyboard activation', () => {
    const action = vi.fn();
    const button = renderButton(action);

    fireEvent.click(button, { detail: 1 });
    fireEvent.click(button, { detail: 1 });
    fireEvent.click(button, { detail: 0 });

    expect(action).toHaveBeenCalledTimes(3);
  });
});
