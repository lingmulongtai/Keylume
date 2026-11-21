import { useEffect, useRef, useState } from 'react';
import { Pause, Play, Square, Upload } from 'lucide-react';
import { getPianoState, native } from './api';
import { demoSongs } from './demo-library';
import { stageCommand, subscribeStage } from './stage/api';
import { importMidi } from './stage/import-midi';
import type { StageSnapshot } from './stage/types';
import type { ViewProps } from './ui-state';

const timestamp = (seconds: number) => {
  const whole = Math.floor(Math.max(0, seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
};

export default function HomeDemo({
  state,
  act,
  toast,
  onKeys,
}: Pick<ViewProps, 'state' | 'act' | 'toast'> & { onKeys: (pitches: number[]) => void }) {
  const [transport, setTransport] = useState<StageSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const pending = useRef<AbortController | null>(null);

  useEffect(() => {
    let dead = false;
    let received = false;
    let previousKeys = '';
    let previousTransport = '';
    let off: (() => void) | undefined;
    const receive = (s: StageSnapshot) => {
      if (dead) return;
      received = true;
      const pitches = s.running ? s.playbackHeld : [];
      const signature = pitches.join(',');
      if (signature !== previousKeys) {
        onKeys(pitches);
        previousKeys = signature;
      }
      const display = [s.songRevision, s.running, Math.floor(s.position), s.settings.mode].join(
        '/',
      );
      if (display !== previousTransport) {
        setTransport(s);
        previousTransport = display;
      }
    };
    void subscribeStage(receive)
      .then(async (unsubscribe) => {
        if (dead) return unsubscribe();
        off = unsubscribe;
        const initial = await stageCommand('state');
        if (!received) receive(initial);
      })
      .catch((e) => {
        if (!dead) toast(String(e));
      });
    return () => {
      dead = true;
      off?.();
      pending.current?.abort();
      pending.current = null;
    };
  }, [onKeys, toast]);

  const current = demoSongs.find((s) => s.title === transport?.title);
  const selected = current?.id ?? (transport?.title ? 'imported' : demoSongs[0].id);

  const run = async (work: (signal: AbortSignal) => Promise<void>) => {
    if (pending.current) return;
    const controller = new AbortController();
    pending.current = controller;
    setBusy(true);
    try {
      await work(controller.signal);
    } catch (e) {
      if (!controller.signal.aborted) toast(String(e));
    } finally {
      if (pending.current === controller) {
        pending.current = null;
        setBusy(false);
      }
    }
  };

  const load = async (file: File, signal: AbortSignal, title?: string) => {
    const song = await importMidi(file, signal);
    signal.throwIfAborted();
    if (!song.tracks.some((track) => !track.percussion))
      throw Error('試聴できる楽器トラックがありません');
    if (title) song.title = title;
    await stageCommand('load', { song });
  };
  const loadDemo = async (id: string, signal: AbortSignal) => {
    const demo = demoSongs.find((s) => s.id === id);
    if (!demo) return;
    const response = await fetch(demo.url, { signal });
    if (!response.ok) throw Error('デモMIDIを読み込めませんでした');
    await load(new File([await response.blob()], demo.id + '.mid'), signal, demo.title);
  };
  const play = async (signal: AbortSignal) => {
    if (!transport?.title) await loadDemo(demoSongs[0].id, signal);
    signal.throwIfAborted();
    if (
      !state.settings.piano.enabled &&
      !(await act('patch_settings', { patch: { piano: { enabled: true } } }))
    )
      return;
    if (native) {
      const deadline = performance.now() + 8000;
      for (;;) {
        signal.throwIfAborted();
        const piano = await getPianoState();
        if (piano.state === 'ready') break;
        if (piano.state === 'error' || performance.now() >= deadline)
          throw Error(
            piano.error || '音源を準備できませんでした。ピアノの出力先を確認してください',
          );
        await new Promise((resolve) => setTimeout(resolve, 80));
      }
    }
    signal.throwIfAborted();
    await stageCommand('settings', {
      patch: { mode: 'practice', practiceMode: 'listen', playbackMode: 'full' },
    });
    signal.throwIfAborted();
    await stageCommand('play');
  };
  return (
    <section className="home-demo" aria-label="音源の試聴">
      <strong>テスト演奏</strong>
      <select
        aria-label="試聴する曲"
        value={selected}
        disabled={busy || !transport}
        onChange={(e) => {
          const id = e.target.value;
          void run((signal) => loadDemo(id, signal));
        }}
      >
        {demoSongs.map((s) => (
          <option key={s.id} value={s.id}>
            {s.title}
          </option>
        ))}
        {selected === 'imported' && <option value="imported">{transport?.title}</option>}
      </select>
      <button
        className="primary"
        disabled={busy || !transport}
        onClick={() =>
          void run(
            transport?.running
              ? async () => {
                  await stageCommand('pause');
                }
              : play,
          )
        }
      >
        {transport?.running ? <Pause size={14} /> : <Play size={14} />}
        {transport?.running ? '一時停止' : '試聴する'}
      </button>
      <button
        aria-label="試聴を停止"
        disabled={busy || !transport?.title}
        onClick={() =>
          void run(async () => {
            await stageCommand('stop');
          })
        }
      >
        <Square size={14} />
      </button>
      <output aria-label="試聴の再生時間">
        {timestamp(transport?.position ?? 0)} / {timestamp(transport?.duration ?? 0)}
      </output>
      <button disabled={busy} onClick={() => fileInput.current?.click()}>
        <Upload size={14} />
        MIDIを開く
      </button>
      <input
        type="file"
        hidden
        accept=".mid,.midi"
        ref={fileInput}
        aria-label="試聴するMIDIファイル"
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = '';
          if (file) void run((signal) => load(file, signal));
        }}
      />
    </section>
  );
}
