import { expect, test } from 'vitest';
import { assignHands, noteHand } from './hands';
import { defaultStageSettings, type Song } from './types';
const song = (pitches: number[], name = 'Piano'): Song => ({
  title: 'test',
  duration: 4,
  beats: [],
  tracks: [{ id: 0, name, channel: 0, percussion: false }],
  notes: pitches.map((pitch, id) => ({
    id,
    pitch,
    start: Math.floor(id / 4),
    end: Math.floor(id / 4) + 0.4,
    velocity: 90,
    track: 0,
  })),
});
test('named hands take priority over register and ambiguous single tracks split chords', () => {
  expect(assignHands(song([72, 76], 'Left Hand')).notes.every((n) => n.hand === 'left')).toBe(true);
  const s = assignHands(song([48, 55, 64, 72, 50, 57, 65, 74]));
  expect(s.notes.map((n) => n.hand)).toEqual([
    'left',
    'left',
    'right',
    'right',
    'left',
    'left',
    'right',
    'right',
  ]);
  expect(noteHand(s.notes[0], { ...defaultStageSettings, trackHands: { 0: 'right' } })).toBe(
    'right',
  );
  expect(
    noteHand(s.notes[2], { ...defaultStageSettings, handStrategy: 'split', splitPitch: 65 }),
  ).toBe('left');
});
