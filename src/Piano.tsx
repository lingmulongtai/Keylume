import { useEffect, useState } from 'react';
import { Power, Minus, Plus, Volume2, Square } from 'lucide-react';
import { command, getPianoState, native, subscribe } from './api';
import type { PianoStatus, PianoSettings } from './types';
import type { ViewProps } from './ui-state';
import { Slider, Toggle } from './components';

export default function Piano({
  state,
  saveSettings,
  toast,
}: Pick<ViewProps, 'state' | 'saveSettings' | 'toast'>) {
  const [status, setStatus] = useState<PianoStatus>({
    state: 'off',
    device: '',
    sampleRate: 0,
    bufferFrames: 0,
    error: '',
    peak: 0,
    muted: false,
  });
  useEffect(() => {
    let dead = false,
      received = false;
    let cleanup: (() => void) | undefined;
    void subscribe<PianoStatus>('piano_state', (s) => {
      received = true;
      if (!dead) setStatus(s);
    })
      .then(async (off) => {
        if (dead) return off();
        cleanup = off;
        const initial = await getPianoState();
        if (!dead && !received) setStatus(initial);
      })
      .catch((error) => {
        if (!dead) toast(String(error));
      });
    return () => {
      dead = true;
      cleanup?.();
    };
  }, [toast]);
  const p = state.settings.piano;
  const update = (change: Partial<PianoSettings>) =>
    saveSettings({ ...state.settings, piano: { ...p, ...change } });
  const label = !native
    ? 'デスクトップ版で発音'
    : !p.enabled
      ? 'オフ'
      : status.muted
        ? 'DAW / スリープ中は消音'
        : status.state === 'ready'
          ? '演奏できます'
          : status.state === 'error'
            ? '出力先を確認してください'
            : '音源を準備中…';
  return (
    <section className="piano-panel" aria-label="内蔵ピアノ">
      <div className="piano-strip">
        <button
          className={`piano-power ${p.enabled ? 'active' : ''}`}
          aria-label={p.enabled ? 'ピアノをオフ' : 'ピアノをオン'}
          aria-pressed={p.enabled}
          onClick={() => update({ enabled: !p.enabled })}
        >
          <Power size={18} />
        </button>
        <div className="piano-name">
          <strong>Upright Piano</strong>
          <small>{label}</small>
        </div>
        <div className="piano-volume">
          <Volume2 size={16} />
          <Slider label="ピアノ音量" value={p.volume} onChange={(volume) => update({ volume })} />
        </div>
        <div className="octave-control">
          <span>Octave</span>
          <button
            aria-label="ピアノを1オクターブ下げる"
            disabled={p.octave <= -3}
            onClick={() => update({ octave: p.octave - 1 })}
          >
            <Minus size={14} />
          </button>
          <output aria-label="ピアノのオクターブ">
            {p.octave > 0 ? '+' : ''}
            {p.octave}
          </output>
          <button
            aria-label="ピアノを1オクターブ上げる"
            disabled={p.octave >= 3}
            onClick={() => update({ octave: p.octave + 1 })}
          >
            <Plus size={14} />
          </button>
        </div>
        <meter aria-label="ピアノ出力レベル" min={0} max={1} value={status.peak} />
        <button
          className="piano-panic"
          onClick={() => void command('piano_panic').catch((e) => toast(String(e)))}
        >
          <Square size={13} />
          全音停止
        </button>
      </div>
      {p.enabled && status.error && (
        <p className="piano-error" role="alert">
          {status.error}
        </p>
      )}
      <details className="piano-details">
        <summary>音声と演奏の設定</summary>
        <div className="piano-options">
          <label className="field">
            <span>音量を操作するフェーダー</span>
            <select
              aria-label="音量を操作するフェーダー"
              value={p.volumeFader}
              onChange={(e) => update({ volumeFader: Number(e.target.value) })}
            >
              <option value={0}>割り当てなし</option>
              {Array.from({ length: 9 }, (_, i) => (
                <option value={i + 1} key={i}>
                  フェーダー {i + 1}
                  {i === 8 ? '（右端）' : ''}
                </option>
              ))}
            </select>
            <small>DAW Volumeモードの位置をピアノ音量へ反映します。</small>
          </label>
          <label className="field">
            <span>ピアノの出力先</span>
            <select
              aria-label="ピアノの出力先"
              value={p.outputDevice}
              onChange={(e) => update({ outputDevice: e.target.value })}
            >
              <option value="">Windows の既定の出力</option>
              {[
                ...new Set([
                  ...state.status.audioDevices,
                  ...(p.outputDevice ? [p.outputDevice] : []),
                ]),
              ].map((d) => (
                <option key={d}>{d}</option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>音声バッファ</span>
            <select
              aria-label="音声バッファ"
              value={p.bufferFrames}
              onChange={(e) => update({ bufferFrames: Number(e.target.value) })}
            >
              {[128, 256, 512, 1024].map((n) => (
                <option key={n} value={n}>
                  {n} samples{n === 128 ? ' · 小さい遅延' : n === 1024 ? ' · 安定性優先' : ''}
                </option>
              ))}
            </select>
          </label>
          <Toggle
            label="DAW 使用中はピアノを消音"
            checked={p.muteWithDaw}
            onChange={(muteWithDaw) => update({ muteWithDaw })}
          />
          <p>
            鍵盤入力: {state.status.keyboard || '未接続'}
            <br />
            {status.sampleRate > 0 &&
              `${status.device} · ${status.sampleRate / 1000} kHz · 実測バッファ ${status.bufferFrames} samples`}
          </p>
          <p>
            本体の Octave ボタンで変えた音程はそのまま演奏できます。上の Octave
            はピアノ音源への追加移調です。変更時は発音中の音を止めます。ペダルは本体の標準サステイン設定（CC64）で使用してください。
          </p>
          <p>
            画面の鍵盤もクリック・Enter / Space
            で演奏できます。ライティングを停止しても、ウィンドウを閉じてもピアノは動作します。音切れがある場合はバッファを大きくしてください。DAW
            に譲るモードでは鍵盤入力も解放します。
          </p>
          <p className="piano-credit">
            FreePats Upright Piano KW small · CC0 1.0 · 128音ポリフォニー
          </p>
        </div>
      </details>
    </section>
  );
}
