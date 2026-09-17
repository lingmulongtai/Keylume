import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { native } from '../api';
import type { ViewProps } from '../ui-state';
const pads = [
  'Low tom',
  'Mid tom',
  'High tom',
  'Rim shot',
  'Ride',
  'Bell',
  'Wood low',
  'Wood high',
  'Kick',
  'Snare',
  'Closed hat',
  'Open hat',
  'Clap',
  'Shaker',
  'Crash',
  'Cowbell',
];
interface LoopStatus {
  mode: string;
  beat: number;
  beats: number;
  count: number;
  bpm: number;
  metronome: boolean;
  full: boolean;
}
export default function Groove({
  state,
  saveSettings,
  toast,
}: Pick<ViewProps, 'state' | 'saveSettings' | 'toast'>) {
  const [loop, setLoop] = useState<LoopStatus>({
      mode: 'stopped',
      beat: 0,
      beats: 8,
      count: 0,
      bpm: 100,
      metronome: true,
      full: false,
    }),
    [bpm, setBpm] = useState(100),
    [bars, setBars] = useState(2),
    [metronome, setMetronome] = useState(true),
    [hit, setHit] = useState(-1);
  const p = state.settings.piano;
  useEffect(() => {
    if (!native) return;
    let dead = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await invoke<LoopStatus>('groove_command', { name: 'state', args: {} });
        if (!dead){setLoop(next);if(next.mode!=='stopped'||next.count>0){setBpm(next.bpm);setBars(next.beats/4);setMetronome(next.metronome);}}
      } catch (e) {
        if (!dead) toast(String(e));
      }
      if (!dead) timer = setTimeout(() => void poll(), 160);
    };
    void poll();
    return () => {
      dead = true;
      clearTimeout(timer);
    };
  }, [toast]);
  const run = (name: string, args: Record<string, unknown> = {}) =>
    void invoke('groove_command', { name, args }).catch((e) => toast(String(e)));
  const active = loop.mode !== 'stopped',
    ready = native && p.enabled;
  return (
    <section className="groove-panel" aria-label="ドラムとルーパー">
      <div className="groove-heading">
        <strong>PAD DRUMS</strong>
        <label>
          <input
            type="checkbox"
            checked={p.drums}
            onChange={(e) =>
              saveSettings({ ...state.settings, piano: { ...p, drums: e.target.checked } })
            }
          />
          パッドで鳴らす
        </label>
        <label>
          ドラム音量
          <input
            aria-label="ドラム音量"
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={p.drumVolume}
            onChange={(e) =>
              saveSettings({
                ...state.settings,
                piano: { ...p, drumVolume: Number(e.target.value) },
              })
            }
          />
        </label>
        <small>
          {ready
            ? '本体のパッド、または下のボタンで演奏'
            : 'ピアノをオンにするとドラムも演奏できます（デスクトップ版）'}
        </small>
      </div>
      <div className="drum-pads">
        {pads.map((name, i) => (
          <button
            key={name}
            disabled={!ready || !p.drums}
            className={hit === i ? 'hit' : ''}
            onPointerDown={() => {
              setHit(i);
              run('drum', { pad: i });
            }}
            onPointerUp={() => setHit(-1)}
            onPointerLeave={() => setHit(-1)}
            onKeyDown={(e) => {
              if ((e.key === 'Enter' || e.key === ' ') && !e.repeat) {
                e.preventDefault();
                setHit(i);
                run('drum', { pad: i });
              }
            }}
            onKeyUp={() => setHit(-1)}
          >
            <small>{i + 1}</small>
            {name}
          </button>
        ))}
      </div>
      <div className="groove-heading">
        <strong>LOOPER</strong>
        <label>
          BPM
          <input
            aria-label="ルーパーのテンポ"
            type="number"
            min={40}
            max={240}
            value={bpm}
            disabled={active || loop.count > 0}
            onChange={(e) => setBpm(Number(e.target.value))}
          />
        </label>
        <label>
          長さ
          <select
            aria-label="ルーパーの小節数"
            value={bars}
            disabled={active || loop.count > 0}
            onChange={(e) => setBars(Number(e.target.value))}
          >
            {[1, 2, 4, 8].map((v) => (
              <option key={v} value={v}>
                {v}小節
              </option>
            ))}
          </select>
        </label>
        <label>
          <input
            type="checkbox"
            checked={metronome}
            disabled={active || loop.count > 0}
            onChange={(e) => setMetronome(e.target.checked)}
          />
          クリック
        </label>
        <output className={loop.mode === 'recording' || loop.mode === 'overdub' ? 'recording' : ''}>
          {
            (
              {
                stopped: '停止',
                countIn: 'カウントイン',
                recording: '録音',
                playing: '再生',
                overdub: '重ね録り',
              } as Record<string, string>
            )[loop.mode]
          }{' '}
          ·{' '}
          {loop.beat < 0
            ? Math.ceil(-loop.beat)
            : `${Math.floor(loop.beat / 4) + 1} / ${loop.beats / 4}`}{' '}
          · {loop.count}イベント
        </output>
      </div>
      <progress aria-label="ループの進行" max={loop.beats} value={Math.max(0, loop.beat)} />
      <div className="stage-toolbar">
        <button
          className="record-button"
          disabled={!ready || active || loop.count > 0}
          onClick={() => run('record', { bpm, bars, metronome })}
        >
          ● 録音
        </button>
        <button disabled={!ready || !loop.count || active} onClick={() => run('play')}>
          ▶ 再生
        </button>
        <button
          aria-pressed={loop.mode === 'overdub'}
          disabled={!ready || !['playing', 'overdub'].includes(loop.mode)}
          onClick={() => run('overdub')}
        >
          {loop.mode === 'overdub' ? '重ね録りを終える' : '重ね録り'}
        </button>
        <button disabled={!ready || !active} onClick={() => run('stop')}>
          ■ 停止
        </button>
        <button disabled={!ready || (!loop.count && !active)} onClick={() => run('clear')}>
          ループを消去
        </button>
        <span>
          4拍のカウント後に録音し、指定の小節で自動再生。ピアノ・ペダル・ドラムを重ねられます。
        </span>
      </div>
      {loop.full && (
        <p role="status">
          録音上限（8,192イベント）です。録音を終了するか、消去してやり直してください。
        </p>
      )}
      <p className="stage-hint">
        ループは今回の起動中のみ保持します。ピアノOFFで消去されます。音色・出力先を手動変更すると停止しますが、録音内容は保持します。テンポ・小節数は新しい録音前に設定してください。物理パッドにはライティング優先でのDAW接続が必要です。
      </p>
    </section>
  );
}
