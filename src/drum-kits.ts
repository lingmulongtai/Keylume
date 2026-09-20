export const drumNames = [
  'Low tom',
  'Mid tom',
  'High tom',
  'Rim shot',
  'Ride',
  'Bell',
  'Wood low',
  'Wood high',
  'Kick',
  'Snare',
  'Closed hat',
  'Open hat',
  'Clap',
  'Shaker',
  'Crash',
  'Cowbell',
];
export const kitNames = ['Studio', 'Sub 808', 'Punch 909', 'Lo-fi', 'Glass', 'Percussion'];
export interface PadSound {
  kind: number;
  tune: number;
  decay: number;
  level: number;
  pan: number;
}
export interface DrumSettings {
  kit: number;
  banks: PadSound[][];
}
export const defaultDrumKit = (): DrumSettings => ({
  kit: 0,
  banks: [
    [0, 1],
    [-3, 1.7],
    [1, 0.8],
    [-5, 0.65],
    [12, 2.2],
    [5, 0.7],
  ].map(([tune, decay]) => drumNames.map((_, kind) => ({ kind, tune, decay, level: 1, pan: 0 }))),
});
