import type { Rect, StageSettings, StageView } from './types';
export function calibrationHandle(
  x: number,
  y: number,
  settings: StageSettings,
  view: StageView,
  width: number,
  height: number,
): 'left' | 'right' | 'lineY' | null {
  const d = view.desktop,
    m = view.monitor;
  const candidates = [
    ['left', Math.abs(x - ((d.x + settings.left * d.width - m.x) / m.width) * width)],
    ['right', Math.abs(x - ((d.x + settings.right * d.width - m.x) / m.width) * width)],
    ['lineY', Math.abs(y - ((d.y + settings.lineY * d.height - m.y) / m.height) * height)],
  ] as const;
  const nearest = [...candidates].sort((a, b) => a[1] - b[1])[0];
  return nearest[1] <= 18 ? nearest[0] : null;
}
export const black = (n: number) => [1, 3, 6, 8, 10].includes(n % 12);
export function keys(low: number, high: number) {
  const result: { pitch: number; x: number; width: number; black: boolean }[] = [];
  let white = 0;
  for (let pitch = low; pitch <= high; pitch++) {
    const dark = black(pitch);
    result.push({ pitch, x: dark ? white - 0.32 : white, width: dark ? 0.64 : 1, black: dark });
    if (!dark) white++;
  }
  const min = Math.min(0, result[0].x),
    max = Math.max(white, result.at(-1)!.x + result.at(-1)!.width);
  return result.map((k) => ({ ...k, x: (k.x - min) / (max - min), width: k.width / (max - min) }));
}
export function union(rects: Rect[]): Rect {
  if (!rects.length) return { x: 0, y: 0, width: 2560, height: 1440 };
  const x = Math.min(...rects.map((r) => r.x)),
    y = Math.min(...rects.map((r) => r.y));
  return {
    x,
    y,
    width: Math.max(...rects.map((r) => r.x + r.width)) - x,
    height: Math.max(...rects.map((r) => r.y + r.height)) - y,
  };
}
export const noteName = (n: number, format: 'english' | 'solfege' = 'english') =>
  (format === 'solfege'
    ? ['ド', 'ド♯', 'レ', 'レ♯', 'ミ', 'ファ', 'ファ♯', 'ソ', 'ソ♯', 'ラ', 'ラ♯', 'シ']
    : ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'])[n % 12] +
  (Math.floor(n / 12) - 1);

export function noteColor(pitch: number, color: string, rainbow: boolean) {
  if (rainbow)
    return `hsl(${(pitch * 29) % 360} ${black(pitch) ? 64 : 78}% ${black(pitch) ? 32 : 76}%)`;
  const rgb = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));
  return `rgb(${rgb.map((n) => Math.round(black(pitch) ? n * 0.46 : n + (255 - n) * 0.32)).join(' ')})`;
}
