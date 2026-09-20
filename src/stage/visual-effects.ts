import { noteColor } from './geometry';
import type { LiveNote, StageSettings } from './types';

type Key = { pitch: number; x: number; width: number; black: boolean };
const noise = (n: number) => {
  const v = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return v - Math.floor(v);
};

/** Deterministic particles share the same physical coordinate space across monitors. */
export function drawNoteEffects(
  ctx: CanvasRenderingContext2D,
  notes: LiveNote[],
  layout: Map<number, Key>,
  settings: StageSettings,
  clock: number,
  left: number,
  width: number,
  line: number,
) {
  if (settings.style === 'clean' || settings.particles === 0) return;
  ctx.save();
  ctx.globalCompositeOperation = 'screen';
  const count = Math.round(8 + settings.particles * 56);
  // Bounded work even during dense imported/physical MIDI and long pedal holds.
  for (const n of notes.slice(-64)) {
    const key = layout.get(n.pitch);
    if (!key || (n.end !== null && clock - n.end > 1.2)) continue;
    const age = Math.max(0, clock - n.start),
      tail = n.end === null ? 1 : Math.max(0, 1 - (clock - n.end) / 1.2);
    const x = left + (key.x + key.width / 2) * width,
      kw = key.width * width;
    const power = 0.35 + (n.velocity / 127) * 0.65;
    const color = noteColor(n.pitch, settings.color, settings.style === 'rainbow');
    ctx.strokeStyle = color;
    ctx.fillStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = settings.style === 'sparks' ? 7 : 14;
    // A soft impact light remains at the strike position even with bars/keyboard hidden.
    ctx.globalAlpha = tail * power * 0.6;
    ctx.beginPath();
    ctx.ellipse(x, line, Math.max(5, kw * 0.65), 5 + power * 4, 0, 0, Math.PI * 2);
    ctx.fill();
    if (settings.style === 'glow') continue;
    if (settings.style === 'laser' || settings.style === 'aurora') {
      const height = Math.min(line, 280 + power * 180),
        g = ctx.createLinearGradient(0, line, 0, line - height);
      g.addColorStop(0, color);
      g.addColorStop(1, 'transparent');
      ctx.fillStyle = g;
      ctx.globalAlpha = tail * (settings.style === 'laser' ? 0.48 : 0.3);
      for (let j = 0; j < 4; j++) {
        ctx.beginPath();
        ctx.moveTo(x - kw * 0.5, line);
        ctx.bezierCurveTo(
          x - kw + Math.sin(age * 2 + j) * 40,
          line - height * 0.4,
          x + Math.cos(age + j) * 70,
          line - height * 0.8,
          x,
          line - height,
        );
        ctx.bezierCurveTo(
          x + kw + Math.cos(age + j) * 50,
          line - height * 0.7,
          x + kw * 0.6,
          line - height * 0.3,
          x + kw * 0.5,
          line,
        );
        ctx.fill();
      }
      if (settings.style === 'laser') {
        ctx.globalAlpha = tail * 0.9;
        ctx.fillRect(x - 1.5, line - height, 3, height);
      }
    }
    for (let i = 0; i < count; i++) {
      const seed = n.id * 43 + i,
        cycle = age * (settings.style === 'snow' ? 0.35 : 0.8) + noise(seed);
      const t = cycle % 1,
        spread = (noise(seed + 1) - 0.5) * 2;
      const px = x + spread * (settings.style === 'flame' ? kw * 0.9 : kw * 1.8 + 90) * t;
      const py = line - t * (160 + noise(seed + 2) * 480) * power;
      ctx.globalAlpha = Math.sqrt(1 - t) * tail * power;
      ctx.lineWidth = 1 + noise(seed + 4) * 2;
      ctx.fillStyle = color;
      ctx.strokeStyle = color;
      if (settings.style === 'rings') {
        if (i > 5) break;
        ctx.beginPath();
        ctx.ellipse(
          x,
          line - t * 110,
          Math.max(1, t * kw * 2.8),
          Math.max(1, t * kw * 0.6),
          0,
          0,
          Math.PI * 2,
        );
        ctx.stroke();
      } else if (settings.style === 'sparks') {
        ctx.beginPath();
        ctx.moveTo(px, py);
        ctx.lineTo(px - spread * 8, py + 5 + t * 13);
        ctx.stroke();
      } else if (settings.style === 'flame') {
        ctx.fillStyle = `hsl(${20 + t * 32} 100% ${55 + noise(seed + 4) * 30}%)`;
        ctx.beginPath();
        ctx.ellipse(
          px + Math.sin(age * 5 + i) * kw * 0.12,
          py,
          Math.max(1, kw * 0.28 * (1 - t)),
          Math.max(2, 24 * (1 - t)),
          0,
          0,
          Math.PI * 2,
        );
        ctx.fill();
      } else if (settings.style === 'snow') {
        ctx.beginPath();
        const sx = px + Math.sin(age * 1.5 + i) * 24;
        ctx.moveTo(sx - 3, py);
        ctx.lineTo(sx + 3, py);
        ctx.moveTo(sx, py - 3);
        ctx.lineTo(sx, py + 3);
        ctx.stroke();
      } else {
        ctx.beginPath();
        ctx.arc(px, py, Math.max(1, (1 - t) * (2 + noise(seed + 3) * 3)), 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }
  ctx.restore();
}
