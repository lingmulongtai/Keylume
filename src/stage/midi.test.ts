import { test, expect } from 'vitest';
import { Midi } from '@tonejs/midi';
import { parseMidi } from './midi';
function bytes(midi: Midi) {
  return Uint8Array.from(midi.toArray()).buffer;
}
test('tempo changes, chord durations and percussion tracks survive import', () => {
  const midi = new Midi();
  midi.header.tempos = [
    { ticks: 0, bpm: 120 },
    { ticks: 480, bpm: 60 },
  ];
  midi.header.update();
  const piano = midi.addTrack();
  piano.name = 'Piano';
  piano
    .addNote({ midi: 60, ticks: 0, durationTicks: 480, velocity: 0.5 })
    .addNote({ midi: 64, ticks: 0, durationTicks: 480 })
    .addNote({ midi: 67, ticks: 480, durationTicks: 480 });
  const drums = midi.addTrack();
  drums.channel = 9;
  drums.addNote({ midi: 36, ticks: 0, durationTicks: 120 });
  const song = parseMidi(bytes(midi), 'song.mid');
  expect(song.notes).toHaveLength(4);
  expect(song.notes.find((n) => n.pitch === 67)).toMatchObject({ start: 0.5, end: 1.5 });
  expect(song.tracks.some((t) => t.percussion)).toBe(true);
  expect(song.beats).toEqual([0, 0.5, 1.5]);
});
test('malformed, empty, SMPTE and sequential files are rejected', () => {
  expect(() => parseMidi(new ArrayBuffer(12), 'bad')).toThrow();
  const midi = new Midi();
  midi.addTrack().addNote({ midi: 60, time: 0, duration: 1 });
  const data = bytes(midi);
  expect(() => parseMidi(data.slice(0, -2), 'cut')).toThrow();
  new DataView(data).setUint16(8, 2);
  expect(() => parseMidi(data, 'sequential')).toThrow(/形式/);
  new DataView(data).setUint16(8, 1);
  new DataView(data).setUint16(12, 0xe728);
  expect(() => parseMidi(data, 'smpte')).toThrow(/SMPTE/);
  expect(() => parseMidi(bytes(new Midi()), 'empty')).toThrow();
});
