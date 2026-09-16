import { mkdirSync, writeFileSync } from 'node:fs';
mkdirSync('resources', { recursive: true });
import { makeLayout } from './generate-layout.mjs';
const layout = makeLayout();
const layer = (effect, params = {}, blend = 'normal', zone = 'all') => ({
  id: crypto.randomUUID(),
  effect,
  params,
  zone,
  opacity: 1,
  blend,
  enabled: true,
});
const definitions = [
  [
    'aurora',
    'オーロラ',
    [layer('aurora', { colors: ['#43ffc2', '#38a7bf', '#a574f9'], speed: 0.6 })],
  ],
  ['spectrum-wave', 'スペクトラムウェーブ', [layer('wave', { speed: 0.5, wavelength: 0.8 })]],
  ['white-breeze', 'ホワイトブリーズ', [layer('breathing', { color: '#fff0d8', period: 6 })]],
  [
    'sunset',
    'サンセット',
    [layer('gradient', { colors: ['#ff883e', '#f65c9d', '#8a5bf7'], angle: 0 })],
  ],
  [
    'neon',
    'ネオンリアクティブ',
    [
      layer('ripple', { color: '#42fbe5', decay: 2, speed: 1 }, 'add'),
      layer('static', { color: '#091635' }),
    ],
  ],
  [
    'starlight',
    '星空',
    [
      layer('starlight', { color: '#c1d7ff', density: 0.3, speed: 0.7 }, 'add'),
      layer('static', { color: '#07172b' }),
    ],
  ],
  ['fire', 'ファイア', [layer('fire', { speed: 1, intensity: 1 })]],
  ['audio-bars', 'オーディオバー', [layer('audio_spectrum', { color: '#52f5be', sensitivity: 1 })]],
  [
    'beat',
    'ビート',
    [
      layer('audio_pulse', { color: '#ff7cb7', sensitivity: 1.8 }, 'add'),
      layer('static', { color: '#10091b' }),
    ],
  ],
  [
    'note-rainbow',
    'キーボードレインボー',
    [layer('note_map', {}, 'add'), layer('static', { color: '#090d17' })],
  ],
  ['night', '夜間', [layer('static', { color: '#ffd3a3' })]],
  ['power-save', '省電力', [layer('hardware_fx', { palette: 76, mode: 'pulse' })]],
  ['vegas', '本体デモ', []],
];
const presets = definitions.map(([id, name, layers]) => ({
  schema: 1,
  id,
  name,
  layers,
  post: {
    brightness: id === 'night' ? 0.15 : id === 'power-save' ? 1 : 0.8,
    saturation: 1,
    temperatureK: 6500,
    gamma: 2.2,
  },
  display: { enabled: false, widget: 'presetName', showOnPresetChange: true },
  builtin: true,
}));
// Approximate preview palette; these colors are explicitly not hardware measurements.
const palette = Array.from({ length: 128 }, (_, i) =>
  i === 0
    ? [0, 0, 0]
    : i === 1
      ? [32, 32, 32]
      : i === 2
        ? [127, 127, 127]
        : i === 3
          ? [255, 255, 255]
          : hsv((i - 4) / 124),
);
function hsv(h) {
  return [0, 2, 1].map((k) =>
    Math.round(
      255 * (1 - Math.max(0, Math.min(1, Math.min((k + h * 6) % 6, 4 - ((k + h * 6) % 6))))),
    ),
  );
}
for (const [file, data] of Object.entries({ layout, presets, palette }))
  writeFileSync(`resources/${file}.json`, JSON.stringify(data, null, 2) + '\n');
