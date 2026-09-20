import { useCallback, useEffect, useRef, useState } from 'react';
import {
  SlidersHorizontal,
  LayoutGrid,
  Workflow,
  Radio,
  Keyboard,
  Settings2,
  Sun,
  Pause,
  Play,
  CircleHelp,
  ChevronRight,
  X,
  Save,
  AlertCircle,
} from 'lucide-react';
import type { AppState, Preset, Settings, Status } from './types';
import { uid } from './types';
import { getState, command, subscribe, native } from './api';
import Editor from './Editor';
import Stage from './stage/Stage';
import Controller from './ControllerScreen';
import {
  PresetsScreen,
  ProfilesScreen,
  CoexistScreen,
  DeviceScreen,
  SettingsScreen,
  SetupWizard,
} from './Screens';
import { Modal } from './components';
import { appVersion } from './version';
import { useUpdates, UpdateBanner } from './Updates';
type Page =
  'stage' | 'lighting' | 'controller' | 'presets' | 'profiles' | 'coexist' | 'device' | 'settings';
const pages = [
  { id: 'lighting', name: 'ライティング', icon: SlidersHorizontal },
  { id: 'stage', name: '演奏', icon: Play },
  { id: 'controller', name: 'コントローラー', icon: Keyboard },
  { id: 'presets', name: 'プリセット', icon: LayoutGrid },
  { id: 'profiles', name: 'プロファイル', icon: Workflow },
  { id: 'coexist', name: '共存設定', icon: Radio },
  { id: 'device', name: 'デバイス', icon: Keyboard },
  { id: 'settings', name: '設定', icon: Settings2 },
] as const;
const statusLabels: Record<string, string> = {
  starting: '接続を準備中',
  preview: 'プレビュー',
  connected: '接続中',
  disconnected: '未接続',
  paused: '一時停止',
  handoff: '待機中 · DAW 使用中',
  unsupported: '未対応モデル',
};
export default function App() {
  const [state, setState] = useState<AppState | null>(null),
    [page, setPage] = useState<Page>('lighting'),
    [notice, setNotice] = useState(''),
    [error, setError] = useState(''),
    [setup, setSetup] = useState(false),
    [saveName, setSaveName] = useState<string | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve()),
    revision = useRef(0),
    timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined),
    pending = useRef<{ name: string; args: Record<string, unknown> } | null>(null);
  const toast = useCallback((message: string) => {
    setNotice(message);
  }, []);
  const updates = useUpdates(toast);
  const act = useCallback(
    (name: string, args: Record<string, unknown> = {}) => {
      if (timer.current) clearTimeout(timer.current);
      const previous = pending.current;
      pending.current = null;
      const r = ++revision.current;
      const operation = queue.current
        .catch(() => {})
        .then(async () => {
          if (previous) await command(previous.name, previous.args);
          await command(name, args);
          const next = await getState();
          if (r === revision.current) setState(next);
          return true;
        })
        .catch((e) => {
          toast(String(e));
          void getState().then(setState);
          return false;
        });
      queue.current = operation;
      return operation;
    },
    [toast],
  );
  const deferred = useCallback(
    (name: string, args: Record<string, unknown>) => {
      revision.current++;
      if (pending.current && pending.current.name !== name) {
        const previous = pending.current;
        pending.current = null;
        queue.current = queue.current
          .catch(() => {})
          .then(() => command(previous.name, previous.args));
      }
      pending.current = { name, args };
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        const p = pending.current;
        pending.current = null;
        if (p) void act(p.name, p.args);
      }, 120);
    },
    [act],
  );
  const edit = useCallback(
    (preset: Preset) => {
      setState((s) => (s ? { ...s, preset } : s));
      deferred('update_preset', { preset });
    },
    [deferred],
  );
  const saveSettings = useCallback(
    (settings: Settings) => {
      setState((s) => (s ? { ...s, settings } : s));
      deferred('save_settings', { settings });
    },
    [deferred],
  );
  useEffect(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];
    void getState()
      .then((s) => {
        if (!disposed) setState(s);
      })
      .catch((e) => setError(String(e)));
    subscribe<Status>('device_status', (status) =>
      setState((s) => (s ? { ...s, status } : s)),
    ).then((fn) => (disposed ? fn() : cleanup.push(fn)));
    subscribe<number>('piano_volume', (volume) =>
      setState((s) =>
        s ? { ...s, settings: { ...s.settings, piano: { ...s.settings.piano, volume } } } : s,
      ),
    ).then((fn) => (disposed ? fn() : cleanup.push(fn)));
    subscribe<
      Pick<Settings, 'piano' | 'controller' | 'masterBrightness' | 'activePreset'> & {
        preset: Preset;
      }
    >('hardware_settings', ({ preset, ...settings }) =>
      setState((s) => (s ? { ...s, settings: { ...s.settings, ...settings }, preset } : s)),
    ).then((fn) => (disposed ? fn() : cleanup.push(fn)));
    subscribe<string>('notice', toast).then((fn) => (disposed ? fn() : cleanup.push(fn)));
    return () => {
      disposed = true;
      cleanup.forEach((fn) => fn());
    };
  }, [toast]);
  useEffect(() => {
    if (!notice) return;
    const id = setTimeout(() => setNotice(''), 5000);
    return () => clearTimeout(id);
  }, [notice]);
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (state)
          setSaveName(state.preset.builtin ? state.preset.name + ' のコピー' : state.preset.name);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [state]);
  if (!state)
    return (
      <div className="loading">
        <img src="/logo.svg" width={60} alt="" />
        <h1>Keylume</h1>
        <p>{error || 'ライティングを準備しています…'}</p>
        {error && <button onClick={() => location.reload()}>再試行</button>}
      </div>
    );
  const props = { state, act, edit, saveSettings, toast };
  const onSave = () =>
    setSaveName(state.preset.builtin ? state.preset.name + ' のコピー' : state.preset.name);
  return (
    <div className="app-shell">
      <header className="app-navigation">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage('lighting');
          }}
          aria-label="Keylume ライティング"
        >
          <img src="/logo.svg" alt="" />
          <span>Keylume</span>
        </a>
        <nav aria-label="メインナビゲーション">
          {pages.map((p) => (
            <button
              key={p.id}
              className={page === p.id ? 'active' : ''}
              aria-current={page === p.id ? 'page' : undefined}
              onClick={() => setPage(p.id)}
            >
              <p.icon size={18} />
              {p.name}
            </button>
          ))}
        </nav>
        <div className="navigation-utilities">
          <button className="help" onClick={() => setSetup(true)}>
            <CircleHelp size={17} />
            セットアップガイド
          </button>
          <span>v{appVersion}</span>
        </div>
      </header>
      <div className="app-content">
        <UpdateBanner updates={updates} />
        <header className="app-header">
          <div className="connection">
            <span className={'connection-dot ' + (state.paused ? 'paused' : '')} />
            <span>
              {statusLabels[state.status.connection] ?? state.status.connection}
              <small>{state.settings.mock ? 'MockDevice' : 'Launchkey MK4 61'}</small>
            </span>
            {!native && <span className="browser-label">Browser preview</span>}
          </div>
          <button className="coexist-status" onClick={() => setPage('coexist')}>
            <Radio size={14} />
            {state.status.effectiveMode === 'handoff' ? 'DAW に譲る' : 'ライティング優先'}
            <ChevronRight size={12} />
          </button>
          <div className="header-divider" />
          {state.settings.controller.enabled && (
            <button
              className="mode-switch"
              aria-label="演奏とデスクトップを切替"
              onClick={() =>
                saveSettings({
                  ...state.settings,
                  controller: {
                    ...state.settings.controller,
                    mode:
                      state.settings.controller.mode === 'performance' ? 'desktop' : 'performance',
                  },
                })
              }
            >
              {state.settings.controller.mode === 'performance'
                ? '演奏モード'
                : 'デスクトップモード'}
            </button>
          )}
          <label className="master-brightness">
            <Sun size={17} />
            <span className="sr-only">マスター輝度</span>
            <input
              type="range"
              min="0"
              max="100"
              value={Math.round(state.settings.masterBrightness * 100)}
              onChange={(e) =>
                saveSettings({ ...state.settings, masterBrightness: Number(e.target.value) / 100 })
              }
            />
            <output className="numeric">
              {Math.round(state.settings.masterBrightness * 100)}%
            </output>
          </label>
          <button
            className={'pause-button ' + (state.paused ? 'is-paused' : '')}
            onClick={() => void act('set_paused', { paused: !state.paused })}
          >
            {state.paused ? <Play size={15} /> : <Pause size={15} />}
            <span>{state.paused ? '再開' : '一時停止'}</span>
          </button>
        </header>
        {!state.settings.setupComplete && (
          <div className="setup-banner">
            <span>
              <Keyboard size={15} />
              プレビューモード · 実機に接続するにはセットアップを開いてください。
            </span>
            <button onClick={() => setSetup(true)}>
              セットアップ
              <ChevronRight size={14} />
            </button>
            <button
              className="icon-button"
              aria-label="案内を閉じる"
              onClick={() => saveSettings({ ...state.settings, setupComplete: true })}
            >
              <X size={14} />
            </button>
          </div>
        )}
        <main className={'main-view ' + (page === 'lighting' ? 'editing' : '')}>
          {page === 'lighting' ? (
            <Editor {...props} onSave={onSave} />
          ) : page === 'stage' ? (
            <Stage {...props} />
          ) : page === 'controller' ? (
            <Controller {...props} />
          ) : page === 'presets' ? (
            <PresetsScreen {...props} />
          ) : page === 'profiles' ? (
            <ProfilesScreen {...props} />
          ) : page === 'coexist' ? (
            <CoexistScreen {...props} />
          ) : page === 'device' ? (
            <DeviceScreen {...props} />
          ) : (
            <SettingsScreen {...props} updates={updates} onSetup={() => setSetup(true)} />
          )}
        </main>
        <footer className="app-footer">
          <span>
            <span className="tiny-dot" />
            {state.paused
              ? 'ライティング停止中'
              : state.status.connection === 'handoff'
                ? 'DAW にポートを解放中'
                : state.settings.mock
                  ? '実機への送信なし · プレビュー'
                  : 'バックグラウンドで動作中'}
          </span>
          <span>
            {state.status.activeProfile ?? '手動プリセット'}
            <i />
            送信 {state.settings.fps} fps
            <i />
            {state.layout.leds.filter((l) => l.kind === 'rgb').length} RGB LED
          </span>
        </footer>
      </div>
      {notice && (
        <div role="status" className="toast">
          <AlertCircle size={18} />
          <span>{notice}</span>
          <button className="icon-button" aria-label="通知を閉じる" onClick={() => setNotice('')}>
            <X size={16} />
          </button>
        </div>
      )}
      {setup && <SetupWizard {...props} onClose={() => setSetup(false)} />}
      {saveName !== null && (
        <Modal title="プリセットを保存" onClose={() => setSaveName(null)}>
          <p className="muted">レイヤー、色補正、OLED 設定をまとめて保存します。</p>
          <label className="field">
            <span>プリセット名</span>
            <input
              autoFocus
              maxLength={80}
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') document.getElementById('save-preset')?.click();
              }}
            />
          </label>
          <button
            id="save-preset"
            className="primary full"
            disabled={!saveName.trim()}
            onClick={async () => {
              const p = {
                ...state.preset,
                id: state.presets.find((p) => p.id === state.preset.id)?.builtin
                  ? uid()
                  : state.preset.id,
                name: saveName.trim(),
                builtin: false,
              };
              if (await act('save_preset', { preset: p })) {
                setSaveName(null);
                toast('プリセットを保存しました');
              }
            }}
          >
            <Save size={16} />
            保存する
          </button>
        </Modal>
      )}
    </div>
  );
}
