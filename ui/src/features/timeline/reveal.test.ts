import { describe, expect, it } from 'vitest';

import { highlightOf, revealOfUndone } from './reveal';

describe('revealOfUndone', () => {
  it('reveals a photo brought back', () => {
    expect(
      revealOfUndone({ kind: 'rejection', id: 4, name: 'IMG_4.jpg', restoredName: null }),
    ).toEqual({ id: 4, reason: 'restored' });
  });

  it('reveals nothing for a photo not on the timeline', () => {
    expect(
      revealOfUndone({ kind: 'rejection', id: null, name: 'IMG_4.jpg', restoredName: null }),
    ).toBeNull();
  });

  it('scrolls to the newest changed photo of an undone edit and highlights all of them', () => {
    expect(
      revealOfUndone({ kind: 'edit', edit: 'cut', name: 'IMG_4.jpg', changed: [7, 3, 9] }),
    ).toEqual({ id: 7, reason: 'restored', also: [7, 3, 9] });
  });

  it('reveals nothing for an edit that changed no photo visibly', () => {
    expect(
      revealOfUndone({ kind: 'edit', edit: 'join', name: 'IMG_4.jpg', changed: [] }),
    ).toBeNull();
  });
});

describe('highlightOf', () => {
  it('highlights the revealed photo and the ones along with it', () => {
    const reveal = { id: 7, reason: 'restored', also: [3] } as const;

    expect(highlightOf(reveal, 7)).toBe('restored');
    expect(highlightOf(reveal, 3)).toBe('restored');
    expect(highlightOf(reveal, 5)).toBeNull();
    expect(highlightOf(null, 7)).toBeNull();
    expect(highlightOf({ id: 2, reason: 'viewerClosed' }, 2)).toBe('viewerClosed');
  });
});
