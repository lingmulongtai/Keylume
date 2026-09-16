import type { Color, Preset, Layout, Layer } from './types';
import { includesLed } from './types';
import palette from '../resources/palette.json';
export const clamp = (n: number, min = 0, max = 1) => Math.max(min, Math.min(max, n));
export const hex = (s: string): Color =>
  /^#[0-9a-f]{6}$/i.test(s)
    ? ([1, 3, 5].map((i) => parseInt(s.slice(i, i + 2), 16) / 255) as Color)
    : [0.25, 1, 0.75];
const mix = (a: Color, b: Color, t: number) => a.map((v, i) => v * (1 - t) + b[i] * t) as Color;
const scale = (c: Color, n: number) => c.map((v) => clamp(v * n)) as Color;
const fract = (n: number) => ((n % 1) + 1) % 1;
const hsv = (h: number): Color =>
  [0, 4, 2].map((n) => 1 - clamp(Math.min((n + h * 6) % 6, 4 - ((n + h * 6) % 6)))) as Color;
const ramp = (colors: Color[], t: number) => {
  const p = clamp(t) * (colors.length - 1);
  const i = Math.floor(p);
  return mix(colors[i], colors[Math.min(i + 1, colors.length - 1)], fract(p));
};
export function compose(a: Color, b: Color, alpha: number, mode: string): Color {
  return a.map((v, i) => {
    const target =
      mode === 'add'
        ? Math.min(1, v + b[i])
        : mode === 'multiply'
          ? v * b[i]
          : mode === 'screen'
            ? 1 - (1 - v) * (1 - b[i])
            : mode === 'max'
              ? Math.max(v, b[i])
              : b[i];
    return clamp(v + (target - v) * alpha);
  }) as Color;
}
interface Hit {
  x: number;
  y: number;
  time: number;
  velocity: number;
  id?: string;
  note: number;
}
const hits: Hit[] = [];
const held = new Set<number>();
export function previewInput(bytes: number[], source: string, layout: Layout) {
  const time = performance.now() / 1000;
  const [s, n, v] = bytes;
  const on = (s & 240) === 144 && v > 0;
  const off = (s & 240) === 128 || ((s & 240) === 144 && v === 0);
  const led =
    source === 'daw'
      ? layout.leds.find(
          (l) =>
            l.group === 'pads' && ((s & 15) === 9 ? l.address.drumNote : l.address.dawNote) === n,
        )
      : undefined;
  if (source === 'keyboard') {
    if (on) held.add(n);
    if (off) held.delete(n);
  }
  if (on) {
    hits.push({
      x: led ? led.pos.x / layout.canvas.w : 0.22 + clamp((n - 36) / 60) * 0.74,
      y: led ? led.pos.y / layout.canvas.h : 0.48,
      time,
      velocity: v / 127,
      id: led?.id,
      note: n,
    });
    if (hits.length > 64) hits.shift();
  }
  return { note: n, ledId: led?.id, pressed: on };
}
const number = (l: Layer, key: string, d: number) =>
  typeof l.params[key] === 'number' && Number.isFinite(l.params[key])
    ? (l.params[key] as number)
    : d;
const text = (l: Layer, key: string, d: string) =>
  typeof l.params[key] === 'string' ? (l.params[key] as string) : d;
function effect(l: Layer, x: number, y: number, i: number, id: string, time: number): Color {
  const color = hex(text(l, 'color', '#43ffc2'));
  const list = Array.isArray(l.params.colors)
    ? l.params.colors.filter((v): v is string => typeof v === 'string').map(hex)
    : [];
  const colors = list.length ? list : [color, hex('#38a7bf'), hex('#a574f9')];
  const speed = number(l, 'speed', 0.6),
    t = time * speed;
  const c = text(l, 'direction', 'right');
  const coord =
    c === 'left'
      ? 1 - x
      : c === 'down'
        ? y
        : c === 'out'
          ? Math.hypot(x - 0.5, y - 0.4)
          : c === 'in'
            ? 1 - Math.hypot(x - 0.5, y - 0.4)
            : x;
  switch (l.effect) {
    case 'static':
      return color;
    case 'paint':
      return hex((l.params.colorsByLed as Record<string, string> | undefined)?.[id] ?? '#000000');
    case 'gradient': {
      const a = (number(l, 'angle', 0) * Math.PI) / 180;
      return ramp(colors, (x - 0.5) * Math.cos(a) + (y - 0.5) * Math.sin(a) + 0.5);
    }
    case 'breathing':
      return scale(
        color,
        ((Math.sin((time / Math.max(0.2, number(l, 'period', 6))) * Math.PI * 2) + 1) / 2) ** 1.5,
      );
    case 'spectrum_cycle':
      return hsv(fract(time / Math.max(0.2, number(l, 'period', 8))));
    case 'wave':
      return hsv(fract(coord / Math.max(0.1, number(l, 'wavelength', 0.8)) - t * 0.1));
    case 'aurora':
      return scale(
        ramp(colors, (Math.sin(x * 6 + t * 0.6) + Math.cos(x * 11 - y * 5 - t * 0.4)) * 0.25 + 0.5),
        0.5 + 0.5 * Math.abs(Math.sin(x * 8 + t)),
      );
    case 'fire':
      return scale(
        ramp(
          [hex('#cc0300'), hex('#ff2903'), hex('#ffbf1f')],
          (Math.sin(x * 31 + t * 2) + Math.sin(x * 57 - t * 4 + y * 17)) * 0.2 + 0.55,
        ),
        number(l, 'intensity', 1),
      );
    case 'starlight': {
      const phase = Math.abs((Math.sin(i * 12.9898) * 43758.547) % 1),
        v = fract(t * 0.3 + phase),
        density = clamp(number(l, 'density', 0.3), 0.02, 1);
      return scale(color, Math.max(0, Math.sin(((v - (1 - density)) / density) * Math.PI)) ** 4);
    }
    case 'reactive':
    case 'ripple': {
      let out: Color = [0, 0, 0];
      for (const h of hits) {
        const age = time - h.time;
        if (age > 10) continue;
        const distance = Math.hypot(x - h.x, y - h.y);
        const v =
          l.effect === 'ripple'
            ? Math.max(0, 1 - Math.abs(distance - age * 0.3 * Math.max(0.1, speed)) / 0.09)
            : h.id === id || (!h.id && Math.abs(x - h.x) < 0.055)
              ? 1
              : 0;
        out = compose(
          out,
          color,
          v * Math.max(0, 1 - age / Math.max(0.1, number(l, 'decay', 2))) * h.velocity,
          'add',
        );
      }
      return out;
    }
    case 'note_map': {
      let out: Color = [0, 0, 0];
      for (const n of held)
        if (Math.abs(x - (0.22 + clamp((n - 36) / 60) * 0.74)) < 0.065)
          out = compose(out, hsv((n % 12) / 12), 1, 'add');
      return out;
    }
    case 'chord_color': {
      let out: Color = [0, 0, 0];
      for (const n of held) {
        const c = hsv((n % 12) / 12);
        out = out.map((v, i) => v + c[i] / held.size) as Color;
      }
      return out;
    }
    case 'tempo_pulse':
    case 'metronome': {
      const beat = (time * number(l, 'bpm', 120)) / 60;
      return scale(
        color,
        Math.max(0, 1 - fract(beat) * 4) *
          (l.effect === 'metronome' && Math.floor(beat) % number(l, 'beats', 4) !== 0 ? 0.25 : 1),
      );
    }
    case 'hardware_fx': {
      const c = palette[Math.round(clamp(number(l, 'palette', 76), 0, 127))].map(
        (v) => v / 255,
      ) as Color;
      const mode = text(l, 'mode', 'pulse');
      return scale(
        c,
        mode === 'pulse'
          ? (Math.sin(time * 2 * Math.PI) + 1) / 2
          : mode === 'flash'
            ? fract(time * 2) < 0.5
              ? 1
              : 0
            : 1,
      );
    }
    default:
      return [0, 0, 0];
  }
}
export function renderPreview(
  preset: Preset,
  layout: Layout,
  time: number,
  master: number,
): Color[] {
  return layout.leds.map((led, i) => {
    let c: Color = [0, 0, 0];
    if (led.kind === 'none') return c;
    for (const l of [...preset.layers].reverse()) {
      if (!l.enabled || !includesLed(l.zone, led)) continue;
      if (
        l.effect === 'paint' &&
        !(l.params.colorsByLed as Record<string, string> | undefined)?.[led.id]
      )
        continue;
      c = compose(
        c,
        effect(l, led.pos.x / layout.canvas.w, led.pos.y / layout.canvas.h, i, led.id, time),
        l.opacity,
        l.blend,
      );
    }
    c = scale(c, preset.post.brightness * master);
    const lum = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
    c = c.map((v) => clamp(lum + (v - lum) * preset.post.saturation)) as Color;
    const warm = clamp((6500 - preset.post.temperatureK) / 5500),
      cool = clamp((preset.post.temperatureK - 6500) / 5500);
    c[0] *= 1 - cool * 0.3;
    c[1] *= 1 - warm * 0.15;
    c[2] *= 1 - warm * 0.65;
    c = c.map((v) => v ** preset.post.gamma) as Color;
    if (led.kind === 'mono')
      c = Array(3).fill(c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722) as Color;
    return c.map((v) => Math.round(v * 127)) as Color;
  });
}
export function ditherImage(data: Uint8ClampedArray, width = 128, height = 64): number[] {
  const pixels = Array.from(
    { length: width * height },
    (_, i) =>
      (data[i * 4] * 0.2126 + data[i * 4 + 1] * 0.7152 + data[i * 4 + 2] * 0.0722) *
      (data[i * 4 + 3] / 255),
  );
  const bits: number[] = [];
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++) {
      const i = y * width + x,
        v = pixels[i] > 127 ? 255 : 0,
        e = pixels[i] - v;
      bits[i] = v ? 1 : 0;
      if (x + 1 < width) pixels[i + 1] += (e * 7) / 16;
      if (y + 1 < height) {
        if (x > 0) pixels[i + width - 1] += (e * 3) / 16;
        pixels[i + width] += (e * 5) / 16;
        if (x + 1 < width) pixels[i + width + 1] += e / 16;
      }
    }
  return bits;
}
