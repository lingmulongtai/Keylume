import type { Song, SongNote, StageSettings } from './types';
export type Hand = 'auto' | 'left' | 'right';
const explicitHand = (name: string): Hand =>
  /(?:left|\blh\b|左手)/i.test(name)
    ? 'left'
    : /(?:right|\brh\b|右手)/i.test(name)
      ? 'right'
      : 'auto';

/** Track labels first; otherwise use pitch proximity and playable chord spans.
 * Hand crossings cannot be inferred reliably from MIDI alone. Manual overrides remain available.
 */
export function assignHands(song: Song): Song {
  const named = new Map(song.tracks.map((t) => [t.id, explicitHand(t.name)]));
  let left = 48,
    right = 72,
    last = -10;
  for (let i = 0; i < song.notes.length;) {
    const start = song.notes[i].start,
      group: SongNote[] = [];
    while (i < song.notes.length && song.notes[i].start < start + 0.04) group.push(song.notes[i++]);
    const notes = group
      .filter(
        (n) =>
          named.get(n.track) === 'auto' && !song.tracks.find((t) => t.id === n.track)?.percussion,
      )
      .sort((a, b) => a.pitch - b.pitch);
    if (start - last > 2) {
      left = 48;
      right = 72;
    }
    const sums = [0],
      leftBias = [0],
      rightBias = [0];
    for (const n of notes) {
      sums.push(sums.at(-1)! + n.pitch);
      leftBias.push(leftBias.at(-1)! + Math.max(0, n.pitch - 63) * 0.5);
      rightBias.push(rightBias.at(-1)! + Math.max(0, 57 - n.pitch) * 0.5);
    }
    let best = 0,
      cost = Infinity;
    for (let split = 0; split <= notes.length; split++) {
      let candidate = 0;
      for (let h = 0; h < 2; h++) {
        const from = h ? split : 0,
          to = h ? notes.length : split,
          count = to - from;
        if (!count) continue;
        const center = (sums[to] - sums[from]) / count;
        candidate += Math.abs(center - (h ? right : left));
        candidate += Math.max(0, notes[to - 1].pitch - notes[from].pitch - 12) * 5;
        candidate += h ? rightBias[to] - rightBias[from] : leftBias[to] - leftBias[from];
        if (count > 5) candidate += (count - 5) * 12;
      }
      if (candidate < cost) {
        cost = candidate;
        best = split;
      }
    }
    notes.forEach((n, index) => {
      n.hand = index < best ? 'left' : 'right';
    });
    for (const n of group) {
      const hand = named.get(n.track);
      if (hand && hand !== 'auto') n.hand = hand;
    }
    const l = group.filter((n) => n.hand === 'left'),
      r = group.filter((n) => n.hand === 'right');
    if (l.length) left = l.reduce((sum, n) => sum + n.pitch, 0) / l.length;
    if (r.length) right = r.reduce((sum, n) => sum + n.pitch, 0) / r.length;
    last = start;
  }
  return song;
}
export function noteHand(n: SongNote, settings: StageSettings): Exclude<Hand, 'auto'> {
  const override = settings.trackHands[String(n.track)];
  if (override && override !== 'auto') return override;
  if (settings.handStrategy === 'auto' && n.hand && n.hand !== 'auto') return n.hand;
  return n.pitch < settings.splitPitch ? 'left' : 'right';
}
export const practiced = (n: SongNote, s: StageSettings) =>
  s.practiceHand === 'both' || noteHand(n, s) === s.practiceHand;
