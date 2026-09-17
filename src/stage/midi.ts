import { Midi } from '@tonejs/midi';
import type { Song } from './types';
export function parseMidi(data: ArrayBuffer, title: string): Song {
  const bytes = new Uint8Array(data),
    view = new DataView(data);
  if (
    bytes.length < 14 ||
    bytes.length > 16 * 1024 * 1024 ||
    String.fromCharCode(...bytes.subarray(0, 4)) !== 'MThd' ||
    view.getUint32(4) !== 6
  )
    throw Error('標準MIDIファイル（.mid / .midi、16 MB以下）を選んでください');
  const format = view.getUint16(8),
    count = view.getUint16(10),
    division = view.getUint16(12);
  if (format > 1 || count === 0 || count > 256 || division === 0 || division & 0x8000)
    throw Error('SMF形式0/1のPPQ MIDIに対応しています（SMPTEと形式2は非対応）');
  let offset = 14;
  for (let track = 0; track < count; track++) {
    if (
      offset + 8 > bytes.length ||
      String.fromCharCode(...bytes.subarray(offset, offset + 4)) !== 'MTrk'
    )
      throw Error('MIDIトラックが破損しています');
    offset += 8 + view.getUint32(offset + 4);
    if (offset > bytes.length) throw Error('MIDIトラックが途中で切れています');
  }
  let midi: Midi;
  try {
    midi = new Midi(data);
  } catch {
    throw Error('MIDIファイルを解析できませんでした');
  }
  const notes: Song['notes'] = [],
    tracks: Song['tracks'] = [];
  midi.tracks.forEach((track, id) => {
    if (!track.notes.length) return;
    tracks.push({
      id,
      name: (track.name || track.instrument.name || `Track ${id + 1}`).slice(0, 200),
      channel: track.channel,
      percussion: track.channel === 9,
    });
    for (const note of track.notes) {
      if (
        !Number.isFinite(note.time) ||
        !Number.isFinite(note.duration) ||
        note.time < 0 ||
        note.duration < 0 ||
        note.time + note.duration > 7200
      )
        throw Error('MIDIの時間が範囲外です（最大2時間）');
      notes.push({
        id: notes.length,
        pitch: note.midi,
        start: note.time,
        end: note.time + Math.max(0.02, note.duration),
        velocity: Math.max(1, Math.round(note.velocity * 127)),
        track: id,
      });
      if (notes.length > 50000) throw Error('MIDIは50,000音まで読み込めます');
    }
  });
  if (!notes.length) throw Error('MIDIに音符がありません');
  notes.sort((a, b) => a.start - b.start || a.pitch - b.pitch);
  const duration = Math.max(...notes.map((n) => n.end));
  if (duration > 7200) throw Error('MIDIは2時間以内にしてください');
  const beats: number[] = [];
  for (let tick = 0; beats.length < 30000; tick += midi.header.ppq) {
    const time = midi.header.ticksToSeconds(tick);
    if (time > duration) break;
    beats.push(time);
  }
  return {
    title: (midi.name || title.replace(/\.midi?$/i, '')).slice(0, 200),
    duration,
    notes,
    tracks,
    beats,
  };
}
export function demoSong(): Song {
  const pitches = [60, 64, 67, 72, 71, 67, 64, 62, 60, 64, 67, 72, 74, 71, 67, 60];
  return {
    title: 'Keylume · はじめの16音',
    duration: 10,
    tracks: [{ id: 0, name: 'Piano', channel: 0, percussion: false }],
    notes: pitches.map((pitch, id) => ({
      id,
      pitch,
      start: 1 + id * 0.5,
      end: 1.4 + id * 0.5,
      velocity: 90,
      track: 0,
    })),
    beats: Array.from({ length: 21 }, (_, i) => i * 0.5),
  };
}
