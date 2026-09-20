import { useCallback, useEffect, useRef, useState } from 'react';
import { Music2, Upload, Play, Pause, Square, Monitor, Maximize2 } from 'lucide-react';
import type { ViewProps } from '../ui-state';
import Piano from '../Piano';
import Groove from './Groove';
import StageCanvas from './Canvas';
import { stageCommand } from './api';
import { useStage } from './useStage';
import { useStageChange } from './useStageChange';
import { demoSong } from './midi';
import { importMidi } from './import-midi';
import { union } from './geometry';
import type { StageSettings, StageMonitor, StageView } from './types';
import './stage.css';
const seconds = (s: number) =>
  `${Math.floor(Math.max(0, s) / 60)}:${Math.floor(Math.max(0, s) % 60)
    .toString()
    .padStart(2, '0')}`;
export default function Stage(props: ViewProps) {
  const { state, song, frame } = useStage(props.toast),
    [monitors, setMonitors] = useState<StageMonitor[]>([]),
    [selected, setSelected] = useState<number[]>([]),
    [calibrate, setCalibrate] = useState(false),
    [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const run = (name: string, args: Record<string, unknown> = {}) =>
    stageCommand(name, args).catch((e) => {
      props.toast(String(e));
      return null;
    });
  const change = useStageChange(props.toast);
  const refresh = useCallback(() => {
    void stageCommand<StageMonitor[]>('monitors')
      .then((ms) => {
        setMonitors(ms);
        setSelected(ms.map((m) => m.id));
      })
      .catch((e) => props.toast(String(e)));
  }, [props.toast]);
  useEffect(refresh, [refresh]);
  const settings = state.settings,
    desktop = union(monitors.filter((m) => selected.includes(m.id))),
    view = { desktop, monitor: { ...desktop, id: -1, name: 'Preview', scale: 1 } };
  const load = async (file: File) => {
    setBusy(true);
    try {
      await stageCommand('load', { song: await importMidi(file) });
    } catch (e) {
      props.toast(String(e));
    } finally {
      setBusy(false);
    }
  };
  const range = (
    label: string,
    key: 'left' | 'right' | 'lineY' | 'lookAhead' | 'trail' | 'particles',
    min: number,
    max: number,
    step: number,
  ) => (
    <label className="stage-field">
      <span>
        {label}
        <output>
          {typeof settings[key] === 'number'
            ? Number(settings[key]).toFixed(
                key === 'left' || key === 'right' || key === 'lineY' ? 2 : 1,
              )
            : ''}
        </output>
      </span>
      <input
        aria-label={label}
        type="range"
        min={min}
        max={max}
        step={step}
        value={settings[key]}
        onChange={(e) => change({ [key]: Number(e.target.value) })}
      />
    </label>
  );
  return (
    <section className="performance-page">
      <header className="screen-title">
        <div>
          <h1>演奏</h1>
          <p>弾く、眺める、曲を覚える。</p>
        </div>
        <div className="stage-mode">
          {(['live', 'practice'] as const).map((mode) => (
            <button
              key={mode}
              aria-pressed={settings.mode === mode}
              className={settings.mode === mode ? 'active' : ''}
              onClick={() => change({ mode })}
            >
              {mode === 'live' ? 'ライブ表示' : 'MIDI練習'}
            </button>
          ))}
        </div>
      </header>
      <Piano {...props} />
      <details className="groove-details">
        <summary>パッドドラムとルーパー</summary>
        <Groove {...props} />
      </details>
      <div className="stage-preview" style={{ aspectRatio: `${desktop.width}/${desktop.height}` }}>
        <StageCanvas frame={frame} song={song} view={view} calibrate={calibrate} change={change} />
      </div>
      <div className="stage-toolbar">
        <button onClick={() => setCalibrate(!calibrate)} aria-pressed={calibrate}>
          <Maximize2 size={15} />
          鍵盤の位置合わせ
        </button>
        <span>
          {calibrate
            ? '両端・横線をドラッグして調整できます'
            : '本体のオクターブと表示音域を合わせてください'}
        </span>
        <button className="primary" onClick={() => void run('open', { ids: selected })}>
          <Monitor size={15} />
          選択した画面に表示
        </button>
        <button onClick={() => void run('close')}>表示を閉じる</button>
      </div>
      {settings.mode === 'practice' && (
        <section className="stage-practice">
          <div className="stage-toolbar">
            <Music2 size={19} />
            <strong>{state.title || 'MIDIファイルを選ぶ'}</strong>
            <span>
              {song
                ? `${song.notes.length.toLocaleString()}音 · ${seconds(song.duration)}`
                : 'テンポ変更・和音に対応'}
            </span>
            <input
              hidden
              ref={input}
              type="file"
              accept=".mid,.midi"
              aria-label="MIDIファイル"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) void load(file);
                e.target.value = '';
              }}
            />
            <button disabled={busy} onClick={() => input.current?.click()}>
              <Upload size={15} />
              {busy ? '読み込み中…' : 'MIDIを読み込む'}
            </button>
            <button onClick={() => void run('load', { song: demoSong() })}>お試し曲</button>
          </div>
          <div className="stage-transport">
            <button
              className="primary"
              disabled={!song}
              aria-label={state.running ? '練習を一時停止' : '練習を開始'}
              onClick={() => void run(state.running ? 'pause' : 'play')}
            >
              {state.running ? <Pause size={18} /> : <Play size={18} />}
            </button>
            <button disabled={!song} aria-label="練習を停止" onClick={() => void run('stop')}>
              <Square size={16} />
            </button>
            <output>
              {seconds(state.position)} / {seconds(state.duration)}
            </output>
            <input
              aria-label="練習の再生位置"
              type="range"
              min={0}
              max={Math.max(0.1, state.duration)}
              step={0.1}
              value={Math.max(0, state.position)}
              onChange={(e) => void run('seek', { position: Number(e.target.value) })}
            />
            <label>
              速度{' '}
              <select
                aria-label="練習速度"
                value={settings.speed}
                onChange={(e) => change({ speed: Number(e.target.value) })}
              >
                {[0.25, 0.5, 0.75, 1, 1.25, 1.5, 2].map((v) => (
                  <option key={v} value={v}>
                    {v}×
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="stage-scores" aria-label="タイミング採点">
            <strong>
              {state.score.points}
              <small>POINTS</small>
            </strong>
            {(['perfect', 'good', 'late', 'miss', 'wrong'] as const).map((k) => (
              <span key={k}>
                {state.score[k]}
                <small>{k.toUpperCase()}</small>
              </span>
            ))}
            <span>
              {state.score.combo}
              <small>COMBO / BEST {state.score.best}</small>
            </span>
          </div>
          <div className="stage-options">
            <label className="stage-field">
              <span>練習の進め方</span>
              <select
                aria-label="練習の進め方"
                value={settings.practiceMode}
                onChange={(e) =>
                  change({ practiceMode: e.target.value as StageSettings['practiceMode'] })
                }
              >
                <option value="timing">曲のテンポで進む・タイミング採点</option>
                <option value="wait">正しい音まで待つ</option>
              </select>
            </label>
            <label className="stage-field">
              <span>入力遅延の補正（ms）</span>
              <input
                type="number"
                min={-500}
                max={500}
                step={10}
                value={settings.latencyMs}
                onChange={(e) => change({ latencyMs: Number(e.target.value) })}
              />
              <small>遅れて判定される場合はプラスへ。待機モードでは補正しません。</small>
            </label>
            <div className="stage-field">
              <label>
                <input
                  type="checkbox"
                  checked={settings.loopEnabled}
                  onChange={(e) => change({ loopEnabled: e.target.checked })}
                />
                区間リピート
              </label>
              <div className="stage-pair">
                <label>
                  A 秒
                  <input
                    aria-label="リピート開始秒"
                    type="number"
                    min={0}
                    max={Math.max(0, settings.loopEnd - 0.1)}
                    step={0.1}
                    value={settings.loopStart}
                    onChange={(e) => change({ loopStart: Number(e.target.value) })}
                  />
                </label>
                <label>
                  B 秒
                  <input
                    aria-label="リピート終了秒"
                    type="number"
                    min={settings.loopStart + 0.1}
                    max={Math.max(0.1, state.duration)}
                    step={0.1}
                    value={settings.loopEnd}
                    onChange={(e) => change({ loopEnd: Number(e.target.value) })}
                  />
                </label>
              </div>
            </div>
          </div>
          {song && (
            <div className="stage-tracks" aria-label="練習トラック">
              {song.tracks.map((t) => (
                <label key={t.id}>
                  <input
                    type="checkbox"
                    checked={settings.tracks.includes(t.id)}
                    onChange={(e) =>
                      change({
                        tracks: e.target.checked
                          ? [...settings.tracks, t.id]
                          : settings.tracks.filter((id) => id !== t.id),
                      })
                    }
                  />
                  {t.name}
                  {t.percussion ? '（ドラム）' : ''}
                </label>
              ))}
            </div>
          )}
          <p className="stage-hint">
            鍵盤を弾くと内蔵音源が鳴ります。PERFECT ±80ms / GOOD ±160ms / LATE
            ±250ms。範囲外の音はWRONG、弾かなかった音はMISS。画面外の音も採点対象です。ブラウザープレビューでは発音・採点は行いません。
          </p>
        </section>
      )}
      <div className="stage-options">
        <fieldset>
          <legend>表示するモニター</legend>
          {monitors.map((m) => (
            <label className="stage-monitor" key={m.id}>
              <input
                type="checkbox"
                checked={selected.includes(m.id)}
                onChange={(e) =>
                  setSelected(
                    e.target.checked ? [...selected, m.id] : selected.filter((id) => id !== m.id),
                  )
                }
              />
              <span>
                画面 {m.id + 1} · {m.width} × {m.height}
                <small>{m.name}</small>
              </span>
            </label>
          ))}
          <button onClick={refresh}>画面を再検出</button>
          <p className="stage-hint">
            複数選ぶと連続した1つの鍵盤になります。Windowsの画面配置を基準に、左右端と高さを調整してください。演奏画面は
            Esc で閉じられます。
          </p>
        </fieldset>
        <fieldset>
          <legend>鍵盤と位置</legend>
          <div className="stage-pair">
            <label>
              最低音
              <input
                aria-label="表示の最低音"
                type="number"
                min={0}
                max={settings.high - 12}
                value={settings.low}
                onChange={(e) => change({ low: Number(e.target.value) })}
              />
            </label>
            <label>
              最高音
              <input
                aria-label="表示の最高音"
                type="number"
                min={settings.low + 12}
                max={127}
                value={settings.high}
                onChange={(e) => change({ high: Number(e.target.value) })}
              />
            </label>
          </div>
          <div className="stage-pair">
            <button onClick={() => change({ low: 36, high: 96 })}>61鍵 C2–C7</button>
            <button onClick={() => change({ low: 21, high: 108 })}>88鍵 A0–C8</button>
          </div>
          {range('鍵盤の左端', 'left', 0, settings.right - 0.1, 0.005)}
          {range('鍵盤の右端', 'right', settings.left + 0.1, 1, 0.005)}
          {range('鍵盤の高さ', 'lineY', 0.25, 0.95, 0.005)}
        </fieldset>
        <fieldset>
          <legend>ノートの演出</legend>
          <label className="stage-field">
            <span>スタイル</span>
            <select
              aria-label="ノートのスタイル"
              value={settings.style}
              onChange={(e) => change({ style: e.target.value as StageSettings['style'] })}
            >
              <option value="clean">シンプル</option>
              <option value="glow">やわらかい光</option>
              <option value="particles">光の粒</option>
              <option value="rainbow">音ごとのカラー＋粒</option>
            </select>
          </label>
          <label className="stage-field">
            <span>ノートの色</span>
            <input
              type="color"
              aria-label="ノートの色"
              value={settings.color}
              onChange={(e) => change({ color: e.target.value })}
            />
          </label>
          {range('先読み（秒）', 'lookAhead', 1.5, 12, 0.5)}
          {range('演奏の軌跡（秒）', 'trail', 2, 16, 1)}
          {range('光の粒の量', 'particles', 0, 1, 0.1)}
          <label>
            <input
              type="checkbox"
              checked={settings.labels}
              onChange={(e) => change({ labels: e.target.checked })}
            />
            音名を表示
          </label>
          <label>
            <input
              type="checkbox"
              checked={settings.guides}
              onChange={(e) => change({ guides: e.target.checked })}
            />
            縦のガイド
          </label>
        </fieldset>
      </div>
    </section>
  );
}
export function StageWindow() {
  const [error, setError] = useState(''),
    { state, song, frame } = useStage(setError),
    [view, setView] = useState<StageView | null>(null),
    [calibrate, setCalibrate] = useState(false);
  const change = useStageChange(setError);
  useEffect(() => {
    void stageCommand<StageView>('view')
      .then(setView)
      .catch((e) => setError(String(e)));
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void stageCommand('close');
      if (e.key.toLowerCase() === 'c') setCalibrate((v) => !v);
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, []);
  return (
    <main className="stage-fullscreen">
      {view && (
        <StageCanvas frame={frame} song={song} view={view} calibrate={calibrate} change={change} />
      )}
      <div className="stage-exit">
        <button onClick={() => setCalibrate(!calibrate)}>C · 位置合わせ</button>
        <button onClick={() => void stageCommand('close')}>Esc · 閉じる</button>
        {state.waiting.length > 0 && <span>正しい音を待っています</span>}
        {error && <span role="alert">{error}</span>}
      </div>
    </main>
  );
}
