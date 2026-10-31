import { useMemo, useState } from 'react';

import { saveFolderMode, saveFolderPosition, loadFolderView } from './folderView';
import { loadGap, saveGap } from './gapSetting';
import type { TimelineMode } from './layout';
import type { SavedPosition } from './useSavedPosition';

/** The gap of the folder at `path`: the one chosen in this view, else its stored one. */
export function useGap(path: string) {
  // Each picked folder keeps its own gap; another folder starts from its stored one.
  const [chosen, setChosen] = useState<{ path: string; gap: number } | null>(null);
  const gap = chosen?.path === path ? chosen.gap : loadGap(path);
  const changeGap = (seconds: number) => {
    setChosen({ path, gap: seconds });
    saveGap(path, seconds);
  };
  return [gap, changeGap] as const;
}

/** The view („Serie” / „Wszystkie”) of the folder at `path`: the one chosen in this view, else its stored one. */
export function useMode(path: string) {
  const [chosen, setChosen] = useState<{ path: string; mode: TimelineMode } | null>(null);
  const mode = chosen?.path === path ? chosen.mode : loadFolderView(path).mode;
  const changeMode = (value: TimelineMode) => {
    setChosen({ path, mode: value });
    saveFolderMode(path, value);
  };
  return [mode, changeMode] as const;
}

/** Where the grid of the folder at `path` keeps its position. */
export function useFolderPosition(path: string): SavedPosition {
  return useMemo(
    () => ({
      load: () => loadFolderView(path).position,
      save: (position) => saveFolderPosition(path, position),
    }),
    [path],
  );
}
