// Original short piano scores for comparing instruments. All positions and durations
// are in quarter notes. Both hand tracks use the acoustic grand program.
import { writeFileSync } from 'node:fs';
import midiPackage from '@tonejs/midi';

function score(title, bpm, meter = 4) {
  const midi = new midiPackage.Midi();
  midi.name = title;
  const ppq = midi.header.ppq;
  midi.header.setTempo(bpm);
  midi.header.timeSignatures = [{ ticks: 0, timeSignature: [meter, 4] }];
  const hand = (name, channel) => {
    const track = midi.addTrack();
    track.name = name;
    track.channel = channel;
    return (pitch, beat, length, velocity) => {
      track.addNote({
        midi: pitch,
        ticks: Math.round(beat * ppq),
        durationTicks: Math.round(length * ppq),
        velocity: velocity / 127,
      });
    };
  };
  return {
    left: hand('Left hand', 0),
    right: hand('Right hand', 1),
    tempo(beat, nextBpm) {
      midi.header.tempos.push({ ticks: Math.round(beat * ppq), bpm: nextBpm });
    },
    save(filename) {
      midi.header.update();
      writeFileSync(new URL(`../public/demos/${filename}.mid`, import.meta.url), midi.toArray());
    },
  };
}

function rainOnGlass() {
  const song = score('Rain on Glass', 84);
  const brokenChords = [
    [50, 57, 64, 66],
    [45, 55, 61, 64],
    [47, 54, 61, 62],
    [43, 55, 59, 66],
    [40, 55, 59, 62],
    [45, 52, 59, 61],
    [43, 54, 59, 62],
  ];
  const melody = [
    [
      [0.5, 69, 1.4],
      [2, 76, 0.75],
      [3, 74, 1],
    ],
    [
      [0, 73, 1.4],
      [1.5, 71, 0.45],
      [2, 69, 1.8],
    ],
    [
      [0.5, 66, 0.9],
      [1.5, 69, 0.45],
      [2, 74, 1.8],
    ],
    [
      [0, 78, 1.6],
      [2, 76, 0.8],
      [3, 71, 0.9],
    ],
    [
      [0, 74, 0.9],
      [1, 78, 0.45],
      [1.5, 79, 1.4],
      [3, 76, 0.95],
    ],
    [
      [0, 73, 1.4],
      [1.5, 71, 0.45],
      [2, 69, 1.9],
    ],
    [
      [0, 71, 0.9],
      [1, 74, 0.9],
      [2, 69, 1.8],
    ],
  ];
  const dynamics = [60, 63, 68, 76, 89, 77, 64];
  brokenChords.forEach((chord, bar) => {
    [0, 1, 2, 3, 1, 2].forEach((degree, i) => {
      const beat = [0, 0.5, 1, 1.5, 2.5, 3][i];
      song.left(chord[degree], bar * 4 + beat, degree === 0 ? 2.4 : 0.91, dynamics[bar] - 14 - i);
    });
    melody[bar].forEach(([beat, pitch, length], i) => {
      song.right(pitch, bar * 4 + beat, length, dynamics[bar] + (i % 2 ? 4 : 0));
    });
  });
  song.tempo(26, 77);
  song.tempo(28, 70);
  [38, 50, 57].forEach((pitch, i) => song.left(pitch, 28 + i * 0.04, 3.8, 46 + i * 2));
  [66, 69, 74].forEach((pitch, i) => song.right(pitch, 28.2 + i * 0.085, 3.7, 53 + 3 * i));
  song.save('rain-on-glass');
}

function pocketSwing() {
  const song = score('Pocket Swing', 132);
  const bars = [
    [
      [43, 47, 50, 54],
      [59, 62, 66],
      [71, 74, 69, 67],
    ],
    [
      [40, 44, 47, 50],
      [56, 62, 67],
      [68, 71, 74, 73],
    ],
    [
      [45, 48, 52, 51],
      [55, 60, 64],
      [72, 69, 76, 74],
    ],
    [
      [38, 42, 45, 46],
      [54, 60, 64],
      [78, 76, 72, 69],
    ],
    [
      [47, 50, 54, 41],
      [57, 62, 66],
      [74, 78, 81, 79],
    ],
    [
      [40, 47, 50, 44],
      [56, 62, 65],
      [77, 74, 71, 68],
    ],
    [
      [45, 48, 38, 42],
      [55, 60, 64],
      [69, 72, 78, 74],
    ],
  ];
  bars.forEach(([bass, chord, line], bar) => {
    const start = bar * 4;
    bass.forEach((pitch, i) => song.left(pitch, start + i, 0.77, [82, 65, 76, 69][i]));
    [2 / 3, 2.67].forEach((beat) => {
      const voicing = bar === 6 && beat > 2 ? [54, 60, 64] : chord;
      voicing.forEach((pitch) => song.right(pitch, start + beat, 0.21, 59));
    });
    const syncopation = bar % 2 ? [0, 1, 5 / 3, 3] : [0, 2 / 3, 2, 11 / 3];
    line.forEach((pitch, i) => {
      song.right(pitch, start + syncopation[i], [0.55, 0.24, 0.63, 0.24][i], [87, 77, 94, 73][i]);
    });
  });
  song.left(43, 28, 1.48, 85);
  [59, 64, 66, 71].forEach((pitch) => song.right(pitch, 28, 1.48, 88));
  song.left(38, 30, 0.32, 74);
  song.left(43, 30 + 2 / 3, 1.25, 84);
  [59, 62, 64, 67].forEach((pitch) => song.right(pitch, 30 + 2 / 3, 1.25, 86));
  song.save('pocket-swing');
}

function lanternWaltz() {
  const song = score('Lantern Waltz', 103, 3);
  const bars = [
    [41, [53, 57], [72, 76, 77]],
    [45, [55, 60], [79, 76, 72]],
    [46, [53, 62], [74, 77, 81]],
    [48, [55, 58], [79, 76, 74]],
    [50, [57, 65], [77, 81, 84]],
    [43, [55, 58], [82, 79, 77]],
    [48, [52, 58], [76, 74, 72]],
  ];
  bars.forEach(([bass, chord, line], bar) => {
    const start = bar * 3;
    song.left(bass, start, 1.4, 60 + (bar % 3) * 4);
    chord.forEach((pitch) => {
      song.left(pitch, start + 1, 0.66, 43);
      song.left(pitch, start + 2, 0.73, 49);
    });
    if (bar % 2 === 0) {
      song.right(line[0], start, 1.42, 69);
      song.right(line[1], start + 1.5, 0.45, 64);
      song.right(line[2], start + 2, 0.96, 78);
    } else {
      song.right(line[0], start, 0.9, 80);
      song.right(line[1], start + 1, 0.9, 70);
      song.right(line[2], start + 2, 0.95, 63);
    }
  });
  song.tempo(18, 95);
  song.tempo(21, 82);
  [41, 53].forEach((pitch) => song.left(pitch, 21, 2.93, 53));
  [69, 72, 77].forEach((pitch, i) => song.right(pitch, 21 + i * 0.075, 2.8, 49 + i * 5));
  song.save('lantern-waltz');
}

rainOnGlass();
pocketSwing();
lanternWaltz();
