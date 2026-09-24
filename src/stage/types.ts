export interface SongNote {
  id: number;
  pitch: number;
  start: number;
  end: number;
  velocity: number;
  track: number;
}
export interface SongTrack {
  id: number;
  name: string;
  channel: number;
  percussion: boolean;
}
export interface Song {
  title: string;
  duration: number;
  notes: SongNote[];
  tracks: SongTrack[];
  beats: number[];
}
export interface StageSettings {
  mode: 'live' | 'practice';
  practiceMode: 'timing' | 'wait';
  speed: number;
  lookAhead: number;
  trail: number;
  low: number;
  high: number;
  left: number;
  right: number;
  lineY: number;
  style:
    | 'clean'
    | 'glow'
    | 'particles'
    | 'rainbow'
    | 'sparks'
    | 'flame'
    | 'aurora'
    | 'rings'
    | 'laser'
    | 'snow';
  color: string;
  labels: boolean;
  labelFormat: 'english' | 'solfege';
  showBars: boolean;
  showKeyboard: boolean;
  showHud: boolean;
  transparent: boolean;
  clickThrough: boolean;
  guides: boolean;
  particles: number;
  latencyMs: number;
  loopEnabled: boolean;
  loopStart: number;
  loopEnd: number;
  tracks: number[];
  monitorIds: number[];
}
export const defaultStageSettings: StageSettings = {
  mode: 'live',
  practiceMode: 'timing',
  speed: 1,
  lookAhead: 4,
  trail: 6,
  low: 36,
  high: 96,
  left: 0.05,
  right: 0.95,
  lineY: 0.82,
  style: 'glow',
  color: '#75c8fa',
  labels: true,
  labelFormat: 'english',
  showBars: true,
  showKeyboard: true,
  showHud: true,
  transparent: false,
  clickThrough: false,
  guides: false,
  particles: 0.6,
  latencyMs: 0,
  loopEnabled: false,
  loopStart: 0,
  loopEnd: 8,
  tracks: [],
  monitorIds: [],
};
export interface LiveNote {
  id: number;
  pitch: number;
  velocity: number;
  start: number;
  end: number | null;
}
export interface Score {
  perfect: number;
  good: number;
  late: number;
  miss: number;
  wrong: number;
  combo: number;
  best: number;
  points: number;
}
export interface Judgement {
  pitch: number;
  result: string;
  offsetMs: number;
  at: number;
}
export interface StageSnapshot {
  clock: number;
  position: number;
  running: boolean;
  waiting: number[];
  nextStop: number | null;
  score: Score;
  live: LiveNote[];
  held: number[];
  judgements: Judgement[];
  settings: StageSettings;
  songRevision: number;
  title: string;
  duration: number;
  loopCount: number;
  targetCount: number;
}
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface StageMonitor extends Rect {
  id: number;
  name: string;
  scale: number;
}
export interface StageView {
  monitor: StageMonitor;
  desktop: Rect;
}
export const emptyScore = (): Score => ({
  perfect: 0,
  good: 0,
  late: 0,
  miss: 0,
  wrong: 0,
  combo: 0,
  best: 0,
  points: 0,
});
export const emptyStage = (): StageSnapshot => ({
  clock: 0,
  position: 0,
  running: false,
  waiting: [],
  nextStop: null,
  score: emptyScore(),
  live: [],
  held: [],
  judgements: [],
  settings: structuredClone(defaultStageSettings),
  songRevision: 0,
  title: '',
  duration: 0,
  loopCount: 0,
  targetCount: 0,
});
