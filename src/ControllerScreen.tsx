import { useEffect, useState } from 'react';
import ControllerMap from './ControllerMap';
import { open } from '@tauri-apps/plugin-dialog';
import type { ViewProps } from './ui-state';
import { command, subscribe, native } from './api';
import { Card, PageTitle, Toggle, Slider } from './components';
import {
  actionNames,
  bindingName,
  controlNames,
  defaultController,
  effectNames,
  effectLabels,
  type Binding,
} from './controller';
export default function ControllerScreen({ state, saveSettings, act, toast }: ViewProps) {
  const c = state.settings.controller;
  const [mode, setMode] = useState<'performance' | 'desktop'>(c.mode),
    [selected, setSelected] = useState('encoder-1'),
    [draft, setDraft] = useState<Binding>(c[c.mode]['encoder-1'] ?? { action: 'none', value: '' }),
    [learning, setLearning] = useState(false),
    [recording, setRecording] = useState(false),
    [search, setSearch] = useState('');
  const controls = [
    ...Array.from({ length: 8 }, (_, i) => ({ id: `encoder-${i + 1}`, label: `ノブ ${i + 1}` })),
    ...Array.from({ length: 9 }, (_, i) => ({
      id: `fader-${i + 1}`,
      label: `フェーダー ${i + 1}`,
    })),
    ...state.layout.leds.map((l) => ({
      id: l.id,
      label:
        controlNames[l.id] ??
        (l.id.startsWith('fbtn.') ? `フェーダー下 ${l.id.split('.')[1]}` : (l.label ?? l.id)),
    })),
    ...['pitch-wheel', 'mod-wheel'].map((id) => ({ id, label: controlNames[id] })),
    ...Object.keys(c[mode])
      .filter((id) => id.startsWith('midi:'))
      .map((id) => ({ id, label: id })),
  ];
  const select = (id: string, m = mode) => {
    setSelected(id);
    setDraft(c[m][id] ?? { action: 'none', value: '' });
    setRecording(false);
  };
  useEffect(() => {
    let off: (() => void) | undefined;
    let dead = false;
    void subscribe<string>('control_learned', (id) => {
      if (dead) return;
      setSelected(id);
      setDraft(c[mode][id] ?? { action: 'none', value: '' });
      setLearning(false);
    }).then((fn) => {
      if (dead) fn();
      else off = fn;
    });
    return () => {
      dead = true;
      off?.();
    };
  }, [c, mode]);
  useEffect(
    () => () => {
      void command('controller_learn', { enabled: false });
    },
    [],
  );
  useEffect(() => {
    if (!learning) return;
    const timer = setTimeout(() => {
      setLearning(false);
      void command('controller_learn', { enabled: false });
    }, 30000);
    return () => clearTimeout(timer);
  }, [learning]);
  const changeAction = (action: string) =>
    setDraft({
      action,
      value:
        action === 'effect'
          ? 'reverb'
          : ['sound', 'kit', 'lighting'].includes(action)
            ? '1'
            : action === 'favorite'
              ? '0'
              : action === 'shortcut'
                ? 'Ctrl+Z'
                : '',
    });
  const capture = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) return;
    const key = event.code.startsWith('Key')
      ? event.code.slice(3)
      : event.code.startsWith('Digit')
        ? event.code.slice(5)
        : event.key === ' '
          ? 'Space'
          : event.key.replace('Arrow', '');
    setDraft({
      action: 'shortcut',
      value: [
        event.ctrlKey ? 'Ctrl' : '',
        event.altKey ? 'Alt' : '',
        event.shiftKey ? 'Shift' : '',
        event.metaKey ? 'Win' : '',
        key,
      ]
        .filter(Boolean)
        .join('+'),
    });
    setRecording(false);
  };
  return (
    <div className="page controller-page">
      <PageTitle
        eyebrow="Launchkey"
        title="コントローラー"
        description="本体の操作を、演奏とデスクトップで使い分けます。"
      />
      <Card>
        <Toggle
          label="Keylumeで本体の操作を受け取る"
          checked={c.enabled}
          onChange={(enabled) => saveSettings({ ...state.settings, controller: { ...c, enabled } })}
        />
        <div className="controller-mode">
          <span>現在のモード</span>
          <button
            aria-pressed={c.mode === 'performance'}
            onClick={() =>
              saveSettings({ ...state.settings, controller: { ...c, mode: 'performance' } })
            }
          >
            演奏モード
          </button>
          <button
            aria-pressed={c.mode === 'desktop'}
            onClick={() =>
              saveSettings({ ...state.settings, controller: { ...c, mode: 'desktop' } })
            }
          >
            デスクトップモード
          </button>
          <small>標準: フェーダー下9で切替</small>
        </div>
        <Slider
          label="ホイールのスクロール速度"
          min={0.1}
          max={4}
          step={0.1}
          value={c.scrollSpeed}
          display={`${c.scrollSpeed.toFixed(1)} ×`}
          onChange={(scrollSpeed) =>
            saveSettings({ ...state.settings, controller: { ...c, scrollSpeed } })
          }
        />
      </Card>
      <Card>
        <h3>本体のOLED</h3>
        <Toggle
          label="操作名と現在の値を本体に表示"
          checked={c.displayFeedback}
          onChange={(displayFeedback) =>
            saveSettings({ ...state.settings, controller: { ...c, displayFeedback } })
          }
        />
        <Slider
          label="操作後の表示時間"
          min={1}
          max={10}
          step={0.5}
          value={c.displaySeconds}
          display={`${c.displaySeconds} 秒`}
          onChange={(displaySeconds) =>
            saveSettings({ ...state.settings, controller: { ...c, displaySeconds } })
          }
        />
        <label className="field">
          <span>操作していない間</span>
          <select
            aria-label="OLEDの待機表示"
            value={c.displayIdle}
            onChange={(e) =>
              saveSettings({
                ...state.settings,
                controller: { ...c, displayIdle: e.target.value as 'blank' | 'preset' },
              })
            }
          >
            <option value="blank">何も表示しない</option>
            <option value="preset">ライティングのOLED設定を表示</option>
          </select>
        </label>
        <small>
          ノブ・音量・切り替え結果を一時表示します。本体の文字仕様に合わせて英数字で表示します。
        </small>
      </Card>
      <ControllerMap
        layout={state.layout}
        selected={selected}
        bindings={c[mode]}
        names={Object.fromEntries(controls.map((c) => [c.id, c.label]))}
        select={select}
        inform={toast}
      />
      <div className="controller-editor">
        <Card>
          <div className="segmented">
            {(['performance', 'desktop'] as const).map((m) => (
              <button
                key={m}
                className={mode === m ? 'selected' : ''}
                onClick={() => {
                  setMode(m);
                  select(selected, m);
                }}
              >
                {m === 'performance' ? '演奏時の割り当て' : 'デスクトップ時の割り当て'}
              </button>
            ))}
          </div>
          <input
            className="controller-search"
            aria-label="本体の操作を検索"
            placeholder="ノブ、パッド、Undo…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <div className="controller-list">
            {controls
              .filter((v) => `${v.label} ${v.id}`.toLowerCase().includes(search.toLowerCase()))
              .map((item) => (
                <button
                  key={item.id}
                  className={selected === item.id ? 'selected' : ''}
                  onClick={() => select(item.id)}
                >
                  <span>{item.label}</span>
                  <small>{bindingName(c[mode][item.id])}</small>
                </button>
              ))}
          </div>
        </Card>
        <Card>
          <h3>{controls.find((v) => v.id === selected)?.label ?? selected}</h3>
          <p className="muted">{mode === 'performance' ? '演奏時' : 'デスクトップ時'}の操作</p>
          <button
            disabled={!native}
            aria-pressed={learning}
            onClick={() => {
              const enabled = !learning;
              setLearning(enabled);
              void command('controller_learn', { enabled }).catch((e) => toast(String(e)));
            }}
          >
            {learning ? '待機を中止' : '本体を操作して選択（MIDI Learn）'}
          </button>
          {learning && (
            <p role="status">
              ボタン・ノブ・パッドを操作してください（30秒）。待機中は割り当てを実行しません。
            </p>
          )}
          <label className="field">
            <span>割り当てる機能</span>
            <select
              aria-label="割り当てる機能"
              value={draft.action}
              onChange={(e) => changeAction(e.target.value)}
            >
              {Object.entries(actionNames).map(([id, name]) => (
                <option key={id} value={id}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          {draft.action === 'effect' && (
            <label className="field">
              <span>エフェクト</span>
              <select
                aria-label="割り当てるエフェクト"
                value={draft.value}
                onChange={(e) => setDraft({ ...draft, value: e.target.value })}
              >
                {effectNames.map((id, i) => (
                  <option key={id} value={id}>
                    {effectLabels[i]}
                  </option>
                ))}
              </select>
            </label>
          )}
          {['sound', 'kit', 'lighting'].includes(draft.action) && (
            <select
              aria-label="切り替える方向"
              value={draft.value}
              onChange={(e) => setDraft({ ...draft, value: e.target.value })}
            >
              <option value="-1">前へ</option>
              <option value="1">次へ</option>
            </select>
          )}
          {draft.action === 'favorite' && (
            <label className="field">
              <span>お気に入りの位置</span>
              <input
                type="number"
                min={1}
                max={128}
                value={Number(draft.value) + 1}
                onChange={(e) => setDraft({ ...draft, value: String(Number(e.target.value) - 1) })}
              />
            </label>
          )}
          {['shortcut', 'open'].includes(draft.action) && (
            <label className="field">
              <span>
                {draft.action === 'shortcut' ? 'ショートカット' : 'URL / .exe の絶対パス'}
              </span>
              <input
                aria-label="割り当ての内容"
                placeholder={
                  draft.action === 'shortcut' ? 'Ctrl+Shift+Z' : 'https://… / C:\\…\\app.exe'
                }
                value={recording ? 'キーの組み合わせを押してください' : draft.value}
                onKeyDown={capture}
                onChange={(e) => !recording && setDraft({ ...draft, value: e.target.value })}
              />
            </label>
          )}
          {draft.action === 'shortcut' && (
            <>
              <button
                onClick={() => {
                  setRecording(!recording);
                  setTimeout(
                    () =>
                      document
                        .querySelector<HTMLInputElement>('[aria-label="割り当ての内容"]')
                        ?.focus(),
                    0,
                  );
                }}
              >
                {recording ? '入力を中止' : 'キーを記録'}
              </button>
              <p className="muted">
                Ctrl / Shift / Alt /
                Win、英数字、F1–F24、矢印キーなど。VolumeUp、VolumeDown、VolumeMute、MediaPlayPauseにも対応。
              </p>
            </>
          )}
          {draft.action === 'open' && (
            <button
              disabled={!native}
              onClick={() =>
                void open({ multiple: false, filters: [{ name: 'アプリ', extensions: ['exe'] }] })
                  .then((path) => path && setDraft({ ...draft, value: path }))
                  .catch((e) => toast(String(e)))
              }
            >
              アプリを選ぶ
            </button>
          )}
          <div className="controller-save">
            <button
              className="primary"
              onClick={() =>
                void act('patch_settings', {
                  patch: {
                    controller: { [mode]: { [selected]: draft } },
                  },
                }).then((ok) => ok && toast('割り当てを保存しました'))
              }
            >
              割り当てを保存
            </button>
            <button
              onClick={() =>
                setDraft(defaultController()[mode][selected] ?? { action: 'none', value: '' })
              }
            >
              標準に戻す
            </button>
          </div>
          <p className="muted">
            ノブはエフェクト、ホイールはスクロール、ボタンはショートカットに適しています。アプリに割り当てた入力はDAWへ転送しません。デスクトップモードでは楽器とルーパーを停止します。
          </p>
          <p className="muted">
            Octave・Settingsや本体のモード切替など、MIDI入力として届かない操作は本体側で動作します。ShiftはDAWポートに通知が届く場合に割り当てられます。Customモードの入力はMIDI
            Learnで選べます。
          </p>
        </Card>
      </div>
    </div>
  );
}
