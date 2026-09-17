import { useEffect, useRef, type RefObject } from 'react';
import { keys, noteName } from './geometry';
import type { Song, StageSnapshot, StageView, StageSettings } from './types';
type Frame = { state: StageSnapshot; received: number };
export default function StageCanvas({
  frame,
  song,
  view,
  calibrate = false,
  change,
}: {
  frame: RefObject<Frame>;
  song: Song | null;
  view: StageView;
  calibrate?: boolean;
  change?: (p: Partial<StageSettings>) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null),
    latest = useRef({ song, view, calibrate, change });
  latest.current = { song, view, calibrate, change };
  useEffect(() => {
    const c = canvas.current!;
    const ctx = c.getContext('2d')!;
    let raf = 0;
    let cachedRange = '',
      layout = keys(36, 96);
    const draw = () => {
      const { state: s, received } = frame.current,
        { song, view, calibrate } = latest.current;
      const bounds = c.getBoundingClientRect(),
        ratio = Math.min(2, window.devicePixelRatio || 1);
      const w = Math.round(bounds.width * ratio),
        h = Math.round(bounds.height * ratio);
      if (c.width !== w || c.height !== h) {
        c.width = w;
        c.height = h;
      }
      if (!w || !h) {
        raf = requestAnimationFrame(draw);
        return;
      }
      const a = s.settings,
        d = view.desktop,
        m = view.monitor;
      ctx.setTransform(
        w / m.width,
        0,
        0,
        h / m.height,
        ((d.x - m.x) * w) / m.width,
        ((d.y - m.y) * h) / m.height,
      );
      ctx.fillStyle = '#080b10';
      ctx.fillRect(m.x - d.x, m.y - d.y, m.width, m.height);
      const left = a.left * d.width,
        width = (a.right - a.left) * d.width,
        line = a.lineY * d.height,
        kh = Math.min(d.height * 0.14, 160),
        elapsed = Math.min(0.15, (performance.now() - received) / 1000),
        clock = s.clock + elapsed;
      let position = s.position + (s.running ? elapsed * a.speed : 0);
      if (s.nextStop !== null && a.practiceMode === 'wait')
        position = Math.min(position, Math.max(s.position, s.nextStop));
      const range = `${a.low}-${a.high}`;
      if (cachedRange !== range) {
        layout = keys(a.low, a.high);
        cachedRange = range;
      }
      const map = new Map(layout.map((k) => [k.pitch, k]));
      const pps = line / (a.mode === 'practice' ? a.lookAhead : a.trail);
      ctx.lineWidth = 1;
      ctx.strokeStyle = '#1c222d';
      if (a.guides)
        for (const k of layout) {
          if (k.black) continue;
          const x = left + k.x * width;
          ctx.beginPath();
          ctx.moveTo(x, 0);
          ctx.lineTo(x, line);
          ctx.stroke();
        }
      if (a.mode === 'practice' && song) {
        for (const beat of song.beats) {
          const y = line - (beat - position) * pps;
          if (y < 0 || y > line) continue;
          ctx.strokeStyle = '#1b2633';
          ctx.beginPath();
          ctx.moveTo(left, y);
          ctx.lineTo(left + width, y);
          ctx.stroke();
        }
      }
      function bar(
        pitch: number,
        start: number,
        end: number,
        velocity: number,
        id: number,
        live: boolean,
      ) {
        const k = map.get(pitch);
        if (!k) return;
        const x = left + k.x * width + 2,
          bw = Math.max(2, k.width * width - 4);
        let top, bottom;
        if (live) {
          top = line - (clock - start) * pps;
          bottom = line - (clock - end) * pps;
        } else {
          top = line - (end - position) * pps;
          bottom = line - (start - position) * pps;
        }
        if (bottom < 0 || top > line) return;
        top = Math.max(-4, top);
        bottom = Math.min(line, bottom);
        const color = a.style === 'rainbow' ? `hsl(${(pitch * 29) % 360} 80% 68%)` : a.color;
        ctx.globalAlpha = 0.55 + (velocity / 127) * 0.45;
        ctx.fillStyle = color;
        ctx.shadowColor = color;
        ctx.shadowBlur = a.style === 'clean' ? 0 : 12;
        ctx.beginPath();
        ctx.roundRect(x, top, bw, Math.max(3, bottom - top), Math.min(6, bw / 3));
        ctx.fill();
        ctx.shadowBlur = 0;
        ctx.globalAlpha = 1;
        if (a.labels && bw > 24 && bottom - top > 24) {
          ctx.fillStyle = '#09121d';
          ctx.font = `600 ${Math.min(18, bw * 0.42)}px sans-serif`;
          ctx.textAlign = 'center';
          ctx.fillText(noteName(pitch), x + bw / 2, Math.min(bottom - 7, top + 23));
        }
        if (live && end >= clock - 0.1 && (a.style === 'particles' || a.style === 'rainbow')) {
          ctx.fillStyle = color;
          for (let i = 0; i < Math.round(12 * a.particles); i++) {
            const t = (((clock - start + i * 0.071 + id * 0.013) % 1) + 1) % 1;
            const dx = Math.sin(id * 19 + i * 17) * bw * 2 * t;
            ctx.globalAlpha = 1 - t;
            ctx.beginPath();
            ctx.arc(
              x + bw / 2 + dx,
              line - t * 150 * (0.5 + (i % 4) / 4),
              Math.max(1, 3 * (1 - t)),
              0,
              Math.PI * 2,
            );
            ctx.fill();
          }
          ctx.globalAlpha = 1;
        }
      }
      if (a.mode === 'practice' && song) {
        for (const n of song.notes) {
          if (n.start > position + a.lookAhead) break;
          if (n.end < position || !a.tracks.includes(n.track)) continue;
          bar(n.pitch, n.start, n.end, n.velocity, n.id, false);
        }
      } else for (const n of s.live) bar(n.pitch, n.start, n.end ?? clock, n.velocity, n.id, true);
      ctx.shadowBlur = 12;
      ctx.shadowColor = a.color;
      ctx.fillStyle = a.color;
      ctx.fillRect(left, line - 2, width, 2);
      ctx.shadowBlur = 0;
      for (const dark of [false, true])
        for (const k of layout) {
          if (k.black !== dark) continue;
          const held = s.held.includes(k.pitch),
            waiting = s.waiting.includes(k.pitch);
          ctx.fillStyle = held ? a.color : waiting ? '#eac17c' : dark ? '#121722' : '#d0d6df';
          const x = left + k.x * width,
            kw = k.width * width;
          ctx.fillRect(x + 1, line + 2, Math.max(1, kw - 2), kh * (dark ? 0.62 : 1));
          if (a.labels && !dark && k.pitch % 12 === 0) {
            ctx.fillStyle = '#535b6a';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText(noteName(k.pitch), x + kw / 2, line + kh - 10);
          }
        }
      ctx.textAlign = 'left';
      ctx.font = '18px sans-serif';
      ctx.fillStyle = '#a1aec0';
      const ox = m.x - d.x + 24,
        oy = m.y - d.y + 32;
      ctx.fillText(
        a.mode === 'live'
          ? 'KEYLUME  /  LIVE'
          : `${s.title || 'MIDI PRACTICE'}   ·   ${s.score.points} pt   ·   ${s.score.combo} combo`,
        ox,
        oy,
      );
      if (a.mode === 'practice' && s.running && s.position < 0) {
        ctx.font = '600 64px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillStyle = '#d4dce8';
        ctx.fillText(String(Math.ceil(-position / a.speed)), left + width / 2, line * 0.5);
      }
      const j = s.judgements.at(-1);
      if (j && clock - j.at < 1) {
        ctx.font = '600 28px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillStyle = j.result === 'miss' || j.result === 'wrong' ? '#f69898' : a.color;
        ctx.globalAlpha = Math.min(1, (1 - (clock - j.at)) * 2);
        ctx.fillText(
          j.result.toUpperCase() +
            (j.result === 'wrong' || j.result === 'miss' ? '' : ` ${Math.round(j.offsetMs)} ms`),
          left + width / 2,
          line * 0.2,
        );
        ctx.globalAlpha = 1;
      }
      if (calibrate) {
        ctx.setLineDash([8, 8]);
        ctx.strokeStyle = '#f0ba76';
        ctx.lineWidth = 3;
        for (const x of [left, left + width]) {
          ctx.beginPath();
          ctx.moveTo(x, 0);
          ctx.lineTo(x, d.height);
          ctx.stroke();
        }
        ctx.beginPath();
        ctx.moveTo(0, line);
        ctx.lineTo(d.width, line);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.font = '18px sans-serif';
        ctx.fillStyle = '#f0ba76';
        ctx.textAlign = 'left';
        ctx.fillText('線をドラッグして鍵盤の両端と高さを合わせる', ox, oy + 30);
      }
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [frame]);
  const drag = useRef<'left' | 'right' | 'lineY' | null>(null);
  const point = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect(),
      v = latest.current.view;
    return {
      x:
        (v.monitor.x - v.desktop.x + ((e.clientX - r.left) / r.width) * v.monitor.width) /
        v.desktop.width,
      y:
        (v.monitor.y - v.desktop.y + ((e.clientY - r.top) / r.height) * v.monitor.height) /
        v.desktop.height,
    };
  };
  return (
    <canvas
      ref={canvas}
      className="stage-canvas"
      aria-label="演奏ノート表示"
      onPointerDown={(e) => {
        if (!calibrate) return;
        const p = point(e),
          s = frame.current.state.settings;
        drag.current =
          Math.abs(p.y - s.lineY) < 0.08
            ? 'lineY'
            : Math.abs(p.x - s.left) < Math.abs(p.x - s.right)
              ? 'left'
              : 'right';
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        const p = point(e),
          s = frame.current.state.settings,
          k = drag.current;
        const value =
          k === 'lineY'
            ? Math.max(0.25, Math.min(0.95, p.y))
            : k === 'left'
              ? Math.max(0, Math.min(s.right - 0.1, p.x))
              : Math.max(s.left + 0.1, Math.min(1, p.x));
        latest.current.change?.({ [k]: value });
      }}
      onPointerUp={() => (drag.current = null)}
      onLostPointerCapture={() => (drag.current = null)}
    />
  );
}
