import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppState, Preset, Settings, Layout, Profile, Status, Color } from './types';
import { effects, uid } from './types';
import layout from '../resources/layout.json';
import presets from '../resources/presets.json';
import { previewInput, renderPreview } from './preview';
import { upgradeLayout } from './layout';
export const native = isTauri();
export const defaults: Settings = {
  schema: 1,
  activePreset: 'aurora',
  masterBrightness: 1,
  fps: 30,
  coexistMode: 'lightingFirst',
  mock: true,
  keyboardReactive: true,
  dawDrum: false,
  autoRepair: true,
  fadeOnRelease: true,
  dawProcesses: [
    'Ableton Live*.exe',
    'FL64.exe',
    'Cubase*.exe',
    'Bitwig Studio.exe',
    'Studio One.exe',
    'reaper.exe',
  ],
  padsPort: 'Keylume Pads',
  controlsPort: 'Keylume Controls',
  forwarding: true,
  aftertouch: true,
  padChannel: 9,
  padNotes: [40, 41, 42, 43, 48, 49, 50, 51, 36, 37, 38, 39, 44, 45, 46, 47],
  controlChannel: null,
  ccMap: {},
  shortcutCcs: [],
  midiLog: false,
  setupComplete: false,
  idleMinutes: 10,
  idleBrightness: 0.15,
  nightEnabled: false,
  nightStart: '23:00',
  nightEnd: '07:00',
  nightBrightness: 0.2,
  manualLock: false,
  language: 'ja',
  audioDevice: '',
};
const defaultStatus: Status = {
  connection: 'preview',
  deviceName: 'MockDevice · Launchkey MK4 61',
  service: 'browser',
  dawActive: [],
  activeProfile: null,
  activePreset: 'aurora',
  effectiveMode: 'lightingFirst',
  padsPort: false,
  controlsPort: false,
  audio: 'stopped',
  inquiry: 'ブラウザープレビュー · MIDI 未接続',
  padMode: 2,
  messages: 0,
  bytes: 0,
  bpm: 120,
  warnings: [],
  monitor: [],
  ports: { inputs: [], outputs: [] },
  audioDevices: [],
  probe: null,
  hardwareVerified: false,
};
let mock: AppState = {
  settings: { ...defaults },
  presets: structuredClone(presets) as Preset[],
  profiles: [],
  layout: structuredClone(layout) as Layout,
  preset: structuredClone(presets[0]) as Preset,
  paused: false,
  status: { ...defaultStatus },
  storagePath: 'ブラウザー内のプレビュー保存領域',
  debug: true,
};
try {
  const saved = JSON.parse(localStorage.getItem('keylume-preview-v1') ?? 'null');
  if (saved?.settings && saved?.presets && saved?.layout) {
    saved.layout = upgradeLayout(validateLayout(saved.layout));
    mock = { ...mock, ...saved, status: { ...defaultStatus } };
    mock.status.activePreset = mock.settings.activePreset;
    mock.status.effectiveMode = mock.settings.coexistMode;
    mock.status.connection = mock.paused ? 'paused' : 'preview';
  }
} catch {
  /* Corrupt preview storage is replaced by defaults. */
}
export function validateLayout(value: unknown): Layout {
  if (!value || typeof value !== 'object') throw Error('レイアウトの形式が不正です');
  const l = structuredClone(value) as Layout;
  const finite = (n: unknown): n is number => typeof n === 'number' && Number.isFinite(n);
  if (
    l.schema !== 1 ||
    l.model !== 'launchkey-mk4-61' ||
    !finite(l.canvas?.w) ||
    l.canvas.w <= 0 ||
    !finite(l.canvas?.h) ||
    l.canvas.h <= 0 ||
    !Array.isArray(l.leds) ||
    !l.leds.length ||
    l.leds.length > 128
  )
    throw Error('レイアウトのサイズが不正です');
  const decor = l.decor as unknown as Record<string, unknown>;
  const fieldsValid = (item: unknown, fields: string[]) => {
    if (!item || typeof item !== 'object') return false;
    return fields.every((key) => {
      const n = (item as Record<string, unknown>)[key];
      return finite(n) && Math.abs(n) <= 10000;
    });
  };
  for (const [key, count, fields] of [
    ['keys', 61, ['note', 'x', 'w']],
    ['encoders', 8, ['x', 'y', 'r']],
    ['faders', 9, ['x', 'y', 'h']],
    ['wheels', 2, ['x', 'y', 'w', 'h']],
  ] as [string, number, string[]][]) {
    const items = decor?.[key];
    if (
      !Array.isArray(items) ||
      items.length !== count ||
      items.some((v) => !fieldsValid(v, fields))
    )
      throw Error('デバイス描画データが不正です');
  }
  if (
    l.decor.keys.some((k) => typeof k.black !== 'boolean') ||
    !fieldsValid(decor.display, ['x', 'y', 'w', 'h'])
  )
    throw Error('鍵盤 / 画面の描画データが不正です');
  if (
    l.geometryRevision != null &&
    (!Number.isInteger(l.geometryRevision) || l.geometryRevision < 0)
  )
    throw Error('レイアウトのリビジョンが不正です');
  if (decor.keybed != null && !fieldsValid(decor.keybed, ['y', 'h', 'blackHeight']))
    throw Error('鍵盤のサイズが不正です');
  if (
    decor.controls != null &&
    (!Array.isArray(decor.controls) ||
      decor.controls.length > 64 ||
      decor.controls.some(
        (c) =>
          !c ||
          typeof c.id !== 'string' ||
          c.id.length > 80 ||
          typeof c.label !== 'string' ||
          c.label.length > 80 ||
          !fieldsValid(c.pos, ['x', 'y']) ||
          !fieldsValid(c.size, ['w', 'h']),
      ))
  )
    throw Error('ボタンの描画データが不正です');
  const ids = new Set<string>();
  for (const led of l.leds) {
    if (
      !led ||
      typeof led.id !== 'string' ||
      !led.id ||
      led.id.length > 80 ||
      (led.label != null && (typeof led.label !== 'string' || led.label.length > 80)) ||
      ids.has(led.id) ||
      !['rgb', 'mono', 'none'].includes(led.kind) ||
      !['pads', 'faderButtons', 'buttons'].includes(led.group) ||
      typeof led.verified !== 'boolean' ||
      !led.address ||
      typeof led.address !== 'object'
    )
      throw Error('LED の定義が不正です');
    ids.add(led.id);
    if (
      (led.kind === 'rgb' && led.address.sysexId == null) ||
      (led.kind === 'mono' && led.address.cc == null) ||
      (led.group === 'pads' && (led.address.dawNote == null || led.address.drumNote == null))
    )
      throw Error('LED のアドレスがありません');
    if (
      !finite(led.pos?.x) ||
      !finite(led.pos?.y) ||
      !finite(led.size?.w) ||
      led.size.w <= 0 ||
      !finite(led.size?.h) ||
      led.size.h <= 0
    )
      throw Error('LED の座標が不正です');
    if (
      [led.address.dawNote, led.address.drumNote, led.address.cc, led.address.sysexId].some(
        (v) => v != null && (!Number.isInteger(v) || v < 0 || v > 127),
      )
    )
      throw Error('MIDI アドレスは 0–127 です');
    if (led.address.monoStatus != null && ![0xb3, 0x93].includes(led.address.monoStatus))
      throw Error('単色ステータスは B3 / 93 です');
  }
  return l;
}
export function validatePreset(value: unknown): Preset {
  if (!value || typeof value !== 'object') throw Error('プリセットの形式が不正です');
  const p = structuredClone(value) as Preset;
  if ((p.schema as number) === 0) {
    p.schema = 1;
    p.post ??= { brightness: 0.8, saturation: 1, temperatureK: 6500, gamma: 2.2 };
  }
  if (
    p.schema !== 1 ||
    typeof p.id !== 'string' ||
    !/^[-\w]{1,80}$/.test(p.id) ||
    typeof p.name !== 'string' ||
    !p.name.trim() ||
    p.name.length > 160 ||
    !Array.isArray(p.layers) ||
    p.layers.length > 32
  )
    throw Error('対応していないプリセットです（schema 1）');
  const ranges: Record<string, [number, number]> = {
    brightness: [0, 1],
    saturation: [0, 2],
    gamma: [0.1, 4],
    temperatureK: [1000, 12000],
  };
  for (const [key, [min, max]] of Object.entries(ranges)) {
    const n = p.post?.[key as keyof Preset['post']];
    if (!Number.isFinite(n) || n < min || n > max) throw Error('ポスト処理の値が範囲外です');
  }
  const ids = new Set<string>();
  for (const l of p.layers) {
    if (
      !l ||
      !effects[l.effect] ||
      !l.params ||
      typeof l.params !== 'object' ||
      !['normal', 'add', 'multiply', 'screen', 'max'].includes(l.blend) ||
      typeof l.id !== 'string' ||
      ids.has(l.id) ||
      !Number.isFinite(l.opacity) ||
      l.opacity < 0 ||
      l.opacity > 1 ||
      typeof l.enabled !== 'boolean' ||
      !(Array.isArray(l.zone)
        ? l.zone.length <= 128 && l.zone.every((id) => typeof id === 'string')
        : ['all', 'pads', 'pads.top', 'pads.bottom', 'faderButtons', 'buttons'].includes(l.zone))
    )
      throw Error('レイヤーの形式が不正です');
    ids.add(l.id);
  }
  if (
    p.display?.imageBits &&
    (p.display.imageBits.length !== 8192 || p.display.imageBits.some((v) => v !== 0 && v !== 1))
  )
    throw Error('OLED 画像の形式が不正です');
  p.builtin = false;
  return p;
}
export async function getState(): Promise<AppState> {
  return native ? invoke('get_state') : structuredClone(mock);
}
type Listener = (payload: unknown) => void;
const handlers = new Map<string, Set<Listener>>();
const emit = (event: string, payload: unknown) => handlers.get(event)?.forEach((fn) => fn(payload));
export async function subscribe<T>(event: string, fn: (payload: T) => void) {
  if (native) return listen<T>(event, (e) => fn(e.payload));
  const listener = fn as Listener;
  const set = handlers.get(event) ?? new Set();
  set.add(listener);
  handlers.set(event, set);
  return () => {
    set.delete(listener);
  };
}
let probeVariant = 0,
  probeStarted = 0;
export function frameNow(): Color[] {
  let f = renderPreview(
    mock.preset,
    mock.layout,
    performance.now() / 1000,
    mock.paused || mock.status.connection === 'handoff' || mock.status.connection === 'disconnected'
      ? 0
      : mock.settings.masterBrightness,
  );
  if (mock.status.probe) {
    f = f.map(() => [0, 0, 0]);
    const i = mock.layout.leds.findIndex((l) => l.id === mock.status.probe);
    if (i >= 0) {
      if (probeVariant === 0) f[i][Math.floor((performance.now() - probeStarted) / 1000) % 3] = 90;
      else f[i] = [90, 90, 90];
    }
  }
  return f;
}
export async function command<T = unknown>(
  name: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) return invoke<T>('command', { name, args });
  switch (name) {
    case 'apply_preset': {
      const p = mock.presets.find((p) => p.id === args.id);
      if (!p) throw Error('プリセットがありません');
      mock.preset = structuredClone(p);
      mock.settings.activePreset = p.id;
      mock.status.activePreset = p.id;
      break;
    }
    case 'update_preset':
      mock.preset = validatePreset(args.preset);
      break;
    case 'save_preset': {
      const p = validatePreset(args.preset);
      if (mock.presets.some((v) => v.id === p.id && v.builtin))
        throw Error('同梱プリセットは複製して保存してください');
      mock.presets = mock.presets.filter((v) => v.id !== p.id).concat(p);
      mock.preset = structuredClone(p);
      mock.settings.activePreset = p.id;
      mock.status.activePreset = p.id;
      break;
    }
    case 'delete_preset': {
      if (mock.presets.find((p) => p.id === args.id)?.builtin)
        throw Error('同梱プリセットは削除できません');
      if (mock.profiles.some((p) => p.presetId === args.id))
        throw Error('先に参照するプロファイルを変更してください');
      mock.presets = mock.presets.filter((p) => p.id !== args.id);
      if (mock.preset.id === args.id) {
        mock.preset = structuredClone(mock.presets[0]);
        mock.settings.activePreset = mock.preset.id;
        mock.status.activePreset = mock.preset.id;
      }
      break;
    }
    case 'import_preset': {
      if (typeof args.json !== 'string' || args.json.length > 2_000_000)
        throw Error('プリセットは 2 MB 以下にしてください');
      const p = validatePreset(JSON.parse(args.json));
      if (mock.presets.some((v) => v.id === p.id)) p.id = uid();
      mock.presets.push(p);
      break;
    }
    case 'export_preset':
      return structuredClone(mock.presets.find((p) => p.id === args.id) ?? mock.preset) as T;
    case 'set_paused':
      mock.paused = Boolean(args.paused);
      mock.status.connection = mock.paused ? 'paused' : 'preview';
      break;
    case 'set_master_brightness':
      mock.settings.masterBrightness = Number(args.value);
      break;
    case 'set_coexist_mode':
      mock.settings.coexistMode = args.mode as Settings['coexistMode'];
      mock.status.effectiveMode = mock.settings.coexistMode;
      break;
    case 'save_settings': {
      const settings = args.settings as Settings;
      if (!settings.mock) throw Error('実機接続はデスクトップ版で利用できます');
      mock.settings = structuredClone(settings);
      mock.status.effectiveMode = mock.settings.coexistMode;
      break;
    }
    case 'save_layout':
      mock.layout = validateLayout(args.layout);
      break;
    case 'save_profile': {
      const p = args.profile as Profile;
      mock.profiles = mock.profiles.filter((v) => v.id !== p.id).concat(structuredClone(p));
      break;
    }
    case 'delete_profile':
      mock.profiles = mock.profiles.filter((p) => p.id !== args.id);
      break;
    case 'run_led_probe':
      mock.status.probe = String(args.id);
      probeVariant = Number(args.variant ?? 0);
      probeStarted = performance.now();
      break;
    case 'stop_led_probe':
      mock.status.probe = null;
      break;
    case 'answer_led_probe': {
      const staged = structuredClone(mock.layout);
      const l = staged.leds.find((l) => l.id === args.id);
      if (l) {
        l.kind = args.kind as typeof l.kind;
        l.verified = false;
        if (args.monoStatus) l.address.monoStatus = Number(args.monoStatus);
      }
      mock.layout = validateLayout(staged);
      mock.status.probe = null;
      break;
    }
    case 'simulate_input': {
      const input = previewInput(args.bytes as number[], String(args.source ?? 'daw'), mock.layout);
      emit('input_event', input);
      const b = args.bytes as number[];
      if (b[0] === 182 && b[1] === 29) mock.status.padMode = b[2];
      mock.status.monitor.unshift(
        `← ${b.map((v) => v.toString(16).padStart(2, '0').toUpperCase()).join(' ')}`,
      );
      mock.status.monitor = mock.status.monitor.slice(0, 100);
      break;
    }
    case 'mock_daw':
      mock.status.dawActive = args.value ? ['Mock DAW'] : [];
      mock.status.connection =
        args.value && mock.settings.coexistMode === 'handoff' ? 'handoff' : 'preview';
      break;
    case 'mock_disconnect':
      mock.status.connection = args.value ? 'disconnected' : 'preview';
      break;
    case 'set_device_feature':
      mock.status.monitor.unshift(
        `→ B6 ${Number(args.cc).toString(16)} ${Number(args.value).toString(16)} (Mock)`,
      );
      break;
    case 'reconnect':
      mock.status.connection = 'preview';
      break;
    case 'tap_tempo':
      break;
    default:
      throw Error(`未対応の操作: ${name}`);
  }
  localStorage.setItem('keylume-preview-v1', JSON.stringify({ ...mock, status: defaultStatus }));
  emit('device_status', mock.status);
  return null as T;
}
export async function exportJson(name: string, data: unknown) {
  const contents = JSON.stringify(data, null, 2);
  if (native) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: name,
      filters: [{ name: 'Keylume JSON', extensions: ['json'] }],
    });
    if (path) await invoke('save_export', { path, contents });
  } else {
    const url = URL.createObjectURL(new Blob([contents], { type: 'application/json' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}
export async function openLink(url: string) {
  if (native) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } else window.open(url, '_blank', 'noopener,noreferrer');
}
