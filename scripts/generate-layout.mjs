import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

// Coordinates follow the official Launchkey MK4 61 straight-top photograph.
// See docs/device-layout.md. MIDI addresses are unchanged from v0.1.0.
export function makeLayout() {
  const w = 1000,
    h = 295;
  const rect = (x, y, width, height) => ({
    pos: { x: +(x * w).toFixed(2), y: +(y * h).toFixed(2) },
    size: { w: +(width * w).toFixed(2), h: +(height * h).toFixed(2) },
  });
  const leds = [];
  for (const [row, y, start, drum] of [
    ['top', 0.204, 0x60, [40, 41, 42, 43, 48, 49, 50, 51]],
    ['bottom', 0.296, 0x70, [36, 37, 38, 39, 44, 45, 46, 47]],
  ])
    for (let i = 0; i < 8; i++)
      leds.push({
        id: `pad.${row}.${i + 1}`,
        label: `Pad ${row === 'top' ? i + 1 : i + 9}`,
        kind: 'rgb',
        group: 'pads',
        ...rect(0.5582 + 0.0269 * i, y, 0.0235, 0.0786),
        address: { dawNote: start + i, drumNote: drum[i], sysexId: start + i },
        verified: false,
      });
  for (let i = 0; i < 9; i++)
    leds.push({
      id: `fbtn.${i + 1}`,
      label: `Fader ${i + 1}`,
      kind: 'rgb',
      group: 'faderButtons',
      ...rect(0.2221 + 0.0269 * i, 0.3244, 0.024, 0.0502),
      address: { cc: 0x25 + i, sysexId: 0x25 + i },
      verified: false,
    });
  const buttons = [
    ['shift', 63, 'Shift', 0.475, 0.116, 0.0245, 0.042],
    ['trackPrevious', 103, '‹', 0.475, 0.204, 0.0245, 0.05],
    ['trackNext', 102, '›', 0.5005, 0.204, 0.0245, 0.05],
    ['encoderUp', 51, '⌃', 0.773, 0.0418, 0.0145, 0.055],
    ['encoderDown', 52, '⌄', 0.773, 0.1003, 0.0145, 0.055],
    ['padUp', 106, '⌃', 0.5406, 0.2023, 0.0145, 0.082],
    ['padDown', 107, '⌄', 0.5406, 0.2893, 0.0145, 0.082],
    ['scene', 104, '›', 0.773, 0.2023, 0.0145, 0.082],
    ['function', 105, 'Function', 0.773, 0.2893, 0.0145, 0.082],
    ['capture', 74, 'Capture MIDI', 0.8028, 0.1405, 0.025, 0.052],
    ['quantise', 75, 'Quantise', 0.8028, 0.1973, 0.025, 0.052],
    ['metronome', 76, 'Metronome', 0.8288, 0.1973, 0.025, 0.052],
    ['undo', 77, 'Undo', 0.8288, 0.1405, 0.025, 0.052],
    ['play', 115, '▶', 0.8028, 0.3211, 0.025, 0.052],
    ['stop', 116, '■', 0.8028, 0.2642, 0.025, 0.052],
    ['record', 117, '●', 0.8288, 0.3211, 0.025, 0.052],
    ['loop', 118, '↻', 0.8288, 0.2642, 0.025, 0.052],
  ];
  for (const [id, cc, label, x, y, width, height] of buttons)
    leds.push({
      id: `btn.${id}`,
      label,
      kind: 'mono',
      group: 'buttons',
      ...rect(x, y, width, height),
      address: { cc, sysexId: cc, monoStatus: 179 },
      verified: false,
    });
  const whiteWidth = (0.9506 * w) / 36;
  let white = 0;
  const keys = [];
  for (let note = 36; note <= 96; note++) {
    const black = [1, 3, 6, 8, 10].includes(note % 12);
    keys.push({
      note,
      x: +(0.0245 * w + white * whiteWidth - (black ? whiteWidth * 0.29 : 0)).toFixed(2),
      w: +(black ? whiteWidth * 0.58 : whiteWidth - 0.7).toFixed(2),
      black,
    });
    if (!black) white++;
  }
  const controls = [
    ['octaveUp', '+', 0.1923, 0.0418, 0.0147, 0.0819],
    ['octaveDown', '−', 0.1923, 0.1923, 0.0147, 0.0819],
    ['settings', 'Settings', 0.5005, 0.116, 0.0245, 0.042],
    ['scale', 'Scale', 0.475, 0.2642, 0.0245, 0.05],
    ['chordMap', 'Chord Map', 0.5005, 0.2642, 0.0245, 0.05],
    ['arp', 'Arp', 0.475, 0.3227, 0.0245, 0.05],
    ['fixedChord', 'Fixed Chord', 0.5005, 0.3227, 0.0245, 0.05],
  ].map(([id, label, x, y, width, height]) => ({ id, label, ...rect(x, y, width, height) }));
  return {
    schema: 1,
    model: 'launchkey-mk4-61',
    geometryRevision: 2,
    canvas: { w, h },
    leds,
    decor: {
      keys,
      keybed: { y: 0.4214 * h, h: 0.5234 * h, blackHeight: 0.338 * h },
      controls,
      encoders: Array.from({ length: 8 }, (_, i) => ({
        x: +((0.57 + 0.0269 * i) * w).toFixed(2),
        y: 0.1003 * h,
        r: 6.2,
      })),
      faders: Array.from({ length: 9 }, (_, i) => ({
        x: +((0.2334 + 0.0269 * i) * w).toFixed(2),
        y: 0.0435 * h,
        h: 0.2057 * h,
      })),
      display: { x: 0.4746 * w, y: 0.0418 * h, w: 0.0514 * w, h: 0.0736 * h },
      wheels: [0.023, 0.0612].map((x) => ({
        x: x * w,
        y: 0.0435 * h,
        w: 0.0132 * w,
        h: 0.2258 * h,
      })),
    },
  };
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  writeFileSync('resources/layout.json', JSON.stringify(makeLayout(), null, 2) + '\n');
