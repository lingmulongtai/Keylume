import { useRef, useState } from 'react';
import { Upload, Play, Pause, Square } from 'lucide-react';
import { stageCommand } from './api';
import { importMidi } from './import-midi';
import { demoSong } from './midi';
import type { Song, StageSettings, StageSnapshot } from './types';
const seconds = (s: number) =>
  `${Math.floor(Math.max(0, s) / 60)}:${Math.floor(Math.max(0, s) % 60)
    .toString()
    .padStart(2, '0')}`;
export default function StageTransport({
  state,
  song,
  change,
  error,
  present,
}: {
  state: StageSnapshot;
  song: Song | null;
  change: (patch: Partial<StageSettings>) => void;
  error: (message: string) => void;
  present?: () => Promise<unknown>;
}) {
  const input = useRef<HTMLInputElement>(null),
    [busy, setBusy] = useState(false);
  const run = async (name: string, args: Record<string, unknown> = {}) => {
    setBusy(true);
    try {
      if (name === 'play') await present?.();
      await stageCommand(name, args);
      if (name === 'load') change({ mode: 'practice' });
    } catch (e) {
      error(String(e));
    } finally {
      setBusy(false);
    }
  };
  const load = async (file: File) => {
    setBusy(true);
    try {
      await stageCommand('load', { song: await importMidi(file) });
      await stageCommand('settings', { patch: { mode: 'practice' } });
    } catch (e) {
      error(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="stage-player" aria-label="演奏の再生操作">
      <div className="stage-toolbar">
        <strong title={state.title}>{state.title || 'MIDIファイルを選ぶ'}</strong>
        <input
          hidden
          ref={input}
          type="file"
          accept=".mid,.midi"
          aria-label="MIDIファイル"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void load(f);
            e.target.value = '';
          }}
        />
        <button disabled={busy} onClick={() => input.current?.click()}>
          <Upload size={15} />
          {busy ? '読み込み中…' : 'MIDIを読み込む'}
        </button>
        <button disabled={busy} onClick={() => void run('load', { song: demoSong() })}>
          お試し曲
        </button>
      </div>
      <div className="stage-transport">
        <button
          className="primary"
          disabled={!song || busy}
          aria-label={state.running ? '練習を一時停止' : '練習を開始'}
          onClick={() => void run(state.running ? 'pause' : 'play')}
        >
          {state.running ? <Pause size={18} /> : <Play size={18} />}
        </button>
        <button disabled={!song || busy} aria-label="練習を停止" onClick={() => void run('stop')}>
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
          onChange={(e) =>
            void stageCommand('seek', { position: Number(e.target.value) }).catch((e) =>
              error(String(e)),
            )
          }
        />
        <label>
          速度{' '}
          <select
            aria-label="練習速度"
            value={state.settings.speed}
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
    </div>
  );
}
