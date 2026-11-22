import { useState } from 'react';

import type { BackgroundWork } from './backgroundWork';
import { CacheFallbackNotice } from './CacheFallbackNotice';
import { CacheSettingsDialog } from './CacheSettingsDialog';
import { useCacheClearing } from './useCacheClearing';
import { useCacheMove } from './useCacheMove';
import { useCacheSettings } from './useCacheSettings';

/**
 * The cache settings dialog, opened from the menu, and the notice that the
 * chosen cache folder is missing.
 */
export function CacheSettingsPanel({ work }: { work: BackgroundWork }) {
  const dialog = useCacheSettings();
  const clearing = useCacheClearing(dialog.remeasure);
  const move = useCacheMove((settings) => {
    dialog.applied(settings);
    dialog.remeasure();
  });
  const [fallbackDismissed, setFallbackDismissed] = useState(false);
  const { settings } = dialog;
  const fallback =
    settings.kind === 'loaded' && settings.settings.location.kind === 'fallback'
      ? settings.settings.location.chosen
      : null;

  return (
    <>
      <CacheSettingsDialog
        open={dialog.open}
        onClose={() => {
          move.dismiss();
          clearing.cancel();
          dialog.close();
        }}
        settings={settings}
        size={dialog.size}
        work={work}
        onLimit={dialog.setLimit}
        clearing={clearing}
        move={move}
      />
      {fallback !== null && !fallbackDismissed && !dialog.open && (
        <CacheFallbackNotice
          chosen={fallback}
          onOpenSettings={dialog.show}
          onDismiss={() => setFallbackDismissed(true)}
        />
      )}
    </>
  );
}
