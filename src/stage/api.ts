import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { emptyStage, type StageSnapshot, type Song } from './types';
const native = isTauri();
let state = emptyStage(),
  song: Song | null = null,
  last = performance.now(),
  started = false,
  id = 0;
const listeners = new Set<(s: StageSnapshot) => void>();
export async function subscribeInteraction(callback: (editing: boolean) => void) {
  if (!native) return () => {};
  return listen<boolean>('stage_interaction', (e) => callback(e.payload));
}
try {
  const saved = localStorage.getItem('keylume-stage');
  if (saved) state.settings = { ...state.settings, ...JSON.parse(saved) };
} catch {
  /* Defaults recover an unreadable preview preference. */
}
function emit() {
  for (const fn of listeners) fn(structuredClone(state));
}
function start() {
  if (started) return;
  started = true;
  setInterval(() => {
    const now = performance.now(),
      dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    state.clock += dt;
    if (state.running) {
      state.position += dt * state.settings.speed;
      if (state.position >= state.duration) {
        state.position = state.duration;
        state.running = false;
      }
    }
    state.live = state.live.filter(
      (n) => n.end === null || state.clock - n.end < state.settings.trail,
    );
    emit();
  }, 33);
}
export function previewStageInput(source: string, bytes: number[], octave = 0) {
  if (native || !['keyboard', 'screen'].includes(source)) return;
  start();
  const kind = bytes[0] & 240,
    pitch = bytes[1] + octave * 12;
  if (kind === 144 && bytes[2] > 0) {
    state.live.push({ id: ++id, pitch, velocity: bytes[2], start: state.clock, end: null });
  } else if (kind === 128 || kind === 144) {
    for (const n of state.live) if (n.pitch === pitch && n.end === null) n.end = state.clock;
  } else if (kind === 176 && bytes[1] === 123) {
    for (const n of state.live) if (n.end === null) n.end = state.clock;
  }
  state.held = state.live.filter((n) => n.end === null).map((n) => n.pitch);
  emit();
}
export async function stageCommand<T = StageSnapshot>(
  name: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) return invoke('stage_command', { name, args });
  if (name === 'interaction') return (args.editing ?? false) as T;
  start();
  if (name === 'song') return structuredClone(song) as T;
  if (name === 'monitors')
    return [
      { id: 0, name: 'ブラウザープレビュー', x: 0, y: 0, width: 2560, height: 1440, scale: 1 },
    ] as T;
  if (name === 'view') return null as T;
  if (name === 'open') throw Error('複数モニター表示はデスクトップ版で利用できます');
  if (name === 'load') {
    song = args.song as Song;
    state.songRevision++;
    state.title = song.title;
    state.duration = song.duration;
    state.position = 0;
    state.running = false;
    state.settings = {
      ...state.settings,
      mode: 'practice',
      tracks: song.tracks.filter((t) => !t.percussion).map((t) => t.id),
      loopEnd: Math.max(0.1, song.duration),
    };
    state.targetCount = song.notes.length;
  }
  if (name === 'settings') {
    state.settings = { ...state.settings, ...(args.patch as object) };
    localStorage.setItem('keylume-stage', JSON.stringify(state.settings));
  }
  if (name === 'play') {
    if (!song) throw Error('MIDIを読み込んでください');
    if (state.position >= state.duration) state.position = 0;
    if (state.position === 0) state.position = -3;
    state.running = true;
  }
  if (name === 'pause') state.running = false;
  if (name === 'stop') {
    state.running = false;
    state.position = 0;
  }
  if (name === 'seek') {
    state.running = false;
    state.position = Number(args.position);
  }
  emit();
  return structuredClone(state) as T;
}
export async function subscribeStage(fn: (s: StageSnapshot) => void) {
  if (native) return listen<StageSnapshot>('stage_state', (e) => fn(e.payload));
  start();
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}
