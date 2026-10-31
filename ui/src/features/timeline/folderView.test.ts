import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { loadFolderView, MAX_FOLDERS, saveFolderMode, saveFolderPosition } from './folderView';

const POSITION = { path: 'Camera\\IMG_1.jpg', takenAt: '2024-06-01T12:00:00', offset: 12 };

describe('folderView', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows a new folder as series, at the top', () => {
    expect(loadFolderView('D:\\Zdjęcia')).toEqual({ mode: 'series', position: null });
  });

  it('remembers the mode and the position per folder', () => {
    saveFolderMode('D:\\Desktop\\Złote gody', 'all');
    saveFolderPosition('D:\\Desktop\\Złote gody', POSITION);
    saveFolderPosition('D:\\Zdjęcia', null);

    expect(loadFolderView('D:\\Desktop\\Złote gody')).toEqual({ mode: 'all', position: POSITION });
    expect(loadFolderView('D:\\Zdjęcia')).toEqual({ mode: 'series', position: null });
  });

  it('ignores broken stored data', () => {
    localStorage.setItem('sito.folderView', '{not json');
    expect(loadFolderView('D:\\Zdjęcia')).toEqual({ mode: 'series', position: null });

    localStorage.setItem(
      'sito.folderView',
      JSON.stringify({ 'D:\\Zdjęcia': { mode: 'grid', position: { path: 3, offset: 'x' } } }),
    );
    expect(loadFolderView('D:\\Zdjęcia')).toEqual({ mode: 'series', position: null });
  });

  it('keeps only the folders used last', () => {
    for (let i = 0; i <= MAX_FOLDERS; i++) saveFolderMode(`D:\\Folder ${i}`, 'all');
    // Using the first folder again keeps it.
    saveFolderMode('D:\\Folder 1', 'all');
    saveFolderMode('D:\\Folder new', 'all');

    const stored = Object.keys(JSON.parse(localStorage.getItem('sito.folderView') ?? '{}'));
    expect(stored).toHaveLength(MAX_FOLDERS);
    expect(stored.slice(-2)).toEqual(['D:\\Folder 1', 'D:\\Folder new']);
    expect(loadFolderView('D:\\Folder 0').mode).toBe('series');
    expect(loadFolderView('D:\\Folder 2').mode).toBe('series');
    expect(loadFolderView('D:\\Folder 3').mode).toBe('all');
  });

  it('ignores storage failures', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('full');
    });
    expect(() => saveFolderPosition('D:\\Zdjęcia', POSITION)).not.toThrow();
  });
});
