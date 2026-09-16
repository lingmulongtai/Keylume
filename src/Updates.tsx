import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { native } from './api';
import { appVersion } from './version';
import type { Settings } from './types';
import { Card, Toggle } from './components';

export interface UpdateState {
  status: 'idle' | 'checking' | 'current' | 'available' | 'error';
  currentVersion: string;
  release: { version: string; tag: string; url: string; prerelease: boolean } | null;
  error: string | null;
  checkedAt: string | null;
  dismissedVersion: string | null;
}
export interface UpdateController {
  state: UpdateState;
  check: () => Promise<void>;
  dismiss: () => Promise<void>;
  open: () => Promise<void>;
}
export function useUpdates(toast: (message: string) => void): UpdateController {
  const [state, setState] = useState<UpdateState>({
    status: 'idle',
    currentVersion: appVersion,
    release: null,
    error: null,
    checkedAt: null,
    dismissedVersion: null,
  });
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<UpdateState>('update_status', (e) => {
      if (!disposed) setState(e.payload);
    })
      .then(async (cleanup) => {
        if (disposed) {
          cleanup();
          return;
        }
        unlisten = cleanup;
        const value = await invoke<UpdateState>('get_update_state');
        if (!disposed) setState(value);
      })
      .catch((e) => {
        if (!disposed) toast(String(e));
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [toast]);
  const run = useCallback(
    async (command: string) => {
      if (!native) {
        toast('更新確認は Windows アプリで利用できます');
        return;
      }
      try {
        await invoke(command);
      } catch (e) {
        toast(String(e));
      }
    },
    [toast],
  );
  return {
    state,
    check: () => run('check_updates'),
    dismiss: () => run('dismiss_update'),
    open: () => run('open_update'),
  };
}

export function UpdateBanner({ updates }: { updates: UpdateController }) {
  const { state, open, dismiss } = updates;
  if (!state.release || state.dismissedVersion === state.release.version) return null;
  return (
    <aside className="update-banner" aria-label="アップデートのお知らせ" role="status">
      <div>
        <strong>Keylume v{state.release.version} が利用できます</strong>
        <span>Windows 版をダウンロードして更新してください。</span>
      </div>
      <button className="primary small" onClick={() => void open()}>
        更新ページを開く
      </button>
      <button className="secondary small" onClick={() => void dismiss()}>
        あとで
      </button>
    </aside>
  );
}

export function UpdatesCard({
  updates,
  settings,
  saveSettings,
  enabled = native,
}: {
  updates: UpdateController;
  settings: Settings;
  saveSettings: (settings: Settings) => void;
  enabled?: boolean;
}) {
  const { state, check, open } = updates;
  const message =
    state.status === 'checking'
      ? '新しいバージョンを確認しています…'
      : state.status === 'current'
        ? '新しいバージョンはありません'
        : state.status === 'available'
          ? `v${state.release!.version} が利用できます`
          : state.status === 'error'
            ? state.error
            : 'まだ確認していません';
  return (
    <Card title="アップデート">
      <Toggle
        label="更新を自動確認"
        description="起動時と12時間ごとに確認します。新しい版があれば案内します。"
        checked={settings.checkForUpdates}
        onChange={(v) => saveSettings({ ...settings, checkForUpdates: v })}
      />
      <Toggle
        label="Preview 版も通知する"
        description="初期設定はオンです。オフの場合は安定版のみ確認します。"
        checked={settings.includePrereleases}
        onChange={(v) => saveSettings({ ...settings, includePrereleases: v })}
      />
      <p className="micro">現在のバージョン: v{state.currentVersion}</p>
      <p role="status" className={state.status === 'error' ? 'update-error' : 'muted'}>
        {enabled ? message : '更新確認は Windows アプリで利用できます。'}
      </p>
      {state.checkedAt && (
        <p className="micro">最終確認: {new Date(state.checkedAt).toLocaleString('ja-JP')}</p>
      )}
      <div className="button-row">
        <button
          className="secondary"
          disabled={!enabled || state.status === 'checking'}
          onClick={() => void check()}
        >
          {state.status === 'checking' ? '確認中…' : '今すぐ確認'}
        </button>
        {state.release && (
          <button className="primary" onClick={() => void open()}>
            更新ページを開く
          </button>
        )}
      </div>
      <p className="micro">
        GitHub の公開リリース情報を取得します。更新はダウンロード後にご自身でインストールできます。
      </p>
    </Card>
  );
}
