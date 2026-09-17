export type Color = [number, number, number];
export interface Led {
  id: string;
  label?: string;
  kind: 'rgb' | 'mono' | 'none';
  group: 'pads' | 'faderButtons' | 'buttons';
  pos: { x: number; y: number };
  size: { w: number; h: number };
  address: {
    dawNote?: number;
    drumNote?: number;
    cc?: number;
    sysexId?: number;
    monoStatus?: number;
  };
  verified: boolean;
}
export interface Layout {
  schema: 1;
  geometryRevision?: number;
  model: string;
  canvas: { w: number; h: number };
  leds: Led[];
  decor: {
    keybed?: { y: number; h: number; blackHeight: number };
    controls?: {
      id: string;
      label: string;
      pos: { x: number; y: number };
      size: { w: number; h: number };
    }[];
    keys: { note: number; x: number; w: number; black: boolean }[];
    encoders: { x: number; y: number; r: number }[];
    faders: { x: number; y: number; h: number }[];
    display: { x: number; y: number; w: number; h: number };
    wheels: { x: number; y: number; w: number; h: number }[];
  };
}
export type Zone =
  'all' | 'pads' | 'pads.top' | 'pads.bottom' | 'faderButtons' | 'buttons' | string[];
export type Blend = 'normal' | 'add' | 'multiply' | 'screen' | 'max';
export interface Layer {
  id: string;
  effect: string;
  params: Record<string, unknown>;
  zone: Zone;
  opacity: number;
  blend: Blend;
  enabled: boolean;
}
export interface DisplaySettings {
  enabled: boolean;
  widget: 'presetName' | 'clock' | 'miniSpectrum' | 'image' | 'off';
  imageBits?: number[];
  showOnPresetChange: boolean;
}
export interface Preset {
  schema: 1;
  id: string;
  name: string;
  layers: Layer[];
  post: { brightness: number; saturation: number; temperatureK: number; gamma: number };
  display?: DisplaySettings;
  builtin?: boolean;
}
export type CoexistMode = 'lightingFirst' | 'handoff';
export interface Profile {
  id: string;
  name: string;
  match: {
    processes?: string[];
    foregroundOnly?: boolean;
    timeRange?: [string, string];
    idleMinutes?: number;
  };
  presetId: string;
  coexistMode: CoexistMode;
  priority: number;
}
export interface PianoSettings {
  enabled: boolean;
  volume: number;
  volumeFader: number;
  octave: number;
  outputDevice: string;
  bufferFrames: number;
  muteWithDaw: boolean;
}
export interface PianoStatus {
  state: string;
  device: string;
  sampleRate: number;
  bufferFrames: number;
  error: string;
  peak: number;
  muted: boolean;
}
export interface Settings {
  schema: number;
  activePreset: string;
  masterBrightness: number;
  fps: number;
  coexistMode: CoexistMode;
  mock: boolean;
  keyboardReactive: boolean;
  dawDrum: boolean;
  autoRepair: boolean;
  fadeOnRelease: boolean;
  dawProcesses: string[];
  padsPort: string;
  controlsPort: string;
  forwarding: boolean;
  aftertouch: boolean;
  padChannel: number;
  padNotes: number[];
  controlChannel: number | null;
  ccMap: Record<string, number>;
  shortcutCcs: number[];
  midiLog: boolean;
  setupComplete: boolean;
  idleMinutes: number;
  idleBrightness: number;
  nightEnabled: boolean;
  nightStart: string;
  nightEnd: string;
  nightBrightness: number;
  manualLock: boolean;
  language: string;
  audioDevice: string;
  checkForUpdates: boolean;
  includePrereleases: boolean;
  piano: PianoSettings;
}
export interface Status {
  connection: string;
  deviceName: string;
  service: string;
  dawActive: string[];
  activeProfile: string | null;
  activePreset: string;
  effectiveMode: CoexistMode;
  padsPort: boolean;
  controlsPort: boolean;
  audio: string;
  keyboard: string;
  inquiry: string;
  padMode: number;
  messages: number;
  bytes: number;
  bpm: number;
  warnings: string[];
  monitor: string[];
  ports: { inputs: string[]; outputs: string[] };
  audioDevices: string[];
  probe: string | null;
  hardwareVerified: boolean;
}
export interface AppState {
  settings: Settings;
  presets: Preset[];
  profiles: Profile[];
  layout: Layout;
  preset: Preset;
  paused: boolean;
  status: Status;
  storagePath: string;
  debug: boolean;
}
export const effects: Record<string, string> = {
  static: '単色',
  paint: 'ペイント',
  gradient: 'グラデーション',
  breathing: '呼吸',
  spectrum_cycle: 'スペクトラム',
  wave: 'ウェーブ',
  ripple: '波紋',
  reactive: 'リアクティブ',
  starlight: '星空',
  fire: '炎',
  aurora: 'オーロラ',
  audio_spectrum: 'スペクトラムバー',
  audio_pulse: 'ビートパルス',
  tempo_pulse: 'テンポ',
  note_map: 'ノートマップ',
  chord_color: 'コードカラー',
  metronome: 'メトロノーム',
  hardware_fx: '省電力',
};
export const zones: Record<string, string> = {
  all: 'すべての LED',
  pads: 'パッド全体',
  'pads.top': 'パッド上段',
  'pads.bottom': 'パッド下段',
  faderButtons: 'フェーダーボタン',
  buttons: 'その他のボタン',
};
export const blends: Record<Blend, string> = {
  normal: '通常',
  add: '加算',
  multiply: '乗算',
  screen: 'スクリーン',
  max: '比較（明）',
};
export const uid = () => crypto.randomUUID();
export function includesLed(zone: Zone, led: Led) {
  return Array.isArray(zone)
    ? zone.includes(led.id)
    : zone === 'all' ||
        zone === led.group ||
        (zone === 'pads.top' && led.id.startsWith('pad.top')) ||
        (zone === 'pads.bottom' && led.id.startsWith('pad.bottom'));
}
