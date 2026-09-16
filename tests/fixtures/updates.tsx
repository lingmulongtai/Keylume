import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  UpdateBanner,
  UpdatesCard,
  type UpdateState,
  type UpdateController,
} from '../../src/Updates';
import { defaults } from '../../src/api';
import '../../src/styles.css';

function Fixture() {
  const [settings, saveSettings] = useState(defaults);
  const [state, setState] = useState<UpdateState>({
    status: 'available',
    currentVersion: '0.2.0',
    release: {
      version: '0.3.0',
      tag: 'v0.3.0',
      prerelease: true,
      url: 'https://github.com/lingmulongtai/Keylume/releases/tag/v0.3.0',
    },
    error: null,
    checkedAt: '2026-09-17T00:00:00Z',
    dismissedVersion: null,
  });
  const [opened, setOpened] = useState('');
  const updates: UpdateController = {
    state,
    dismiss: async () => setState((s) => ({ ...s, dismissedVersion: s.release!.version })),
    open: async () => setOpened(state.release!.url),
    check: async () => {
      setState((s) => ({ ...s, status: 'checking' }));
      await new Promise((resolve) => setTimeout(resolve, 400));
      setState((s) => ({ ...s, status: 'error', error: '更新の通信がタイムアウトしました' }));
    },
  };
  return (
    <>
      <UpdateBanner updates={updates} />
      <UpdatesCard updates={updates} settings={settings} saveSettings={saveSettings} enabled />
      <output aria-label="opened URL">{opened}</output>
      <output aria-label="automatic checks">{String(settings.checkForUpdates)}</output>
    </>
  );
}
createRoot(document.getElementById('root')!).render(<Fixture />);
