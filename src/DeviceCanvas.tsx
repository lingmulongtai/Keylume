import { useEffect, useRef, useState } from 'react';
import type { PointerEvent } from 'react';
import type { AppState, Color, Led } from './types';
import { native, frameNow, subscribe } from './api';
interface Props {
  state: AppState;
  selected: string[];
  onSelect: (ids: string[]) => void;
  onPaint?: (ids: string[]) => void;
  onInput: (bytes: number[], source: string) => void;
  paint: boolean;
  zoom: number;
}
export default function DeviceCanvas({
  state,
  selected,
  onSelect,
  onPaint,
  onInput,
  paint,
  zoom,
}: Props) {
  const canvas = useRef<HTMLCanvasElement>(null),
    svg = useRef<SVGSVGElement>(null),
    data = useRef(state),
    frame = useRef<Color[]>([]),
    drag = useRef<{ x: number; y: number; ids: string[] } | null>(null);
  const [held, setHeld] = useState<Set<string>>(new Set()),
    [rect, setRect] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  data.current = state;
  useEffect(() => {
    let dead = false;
    const clean: (() => void)[] = [];
    subscribe<Color[]>('frame_preview', (f) => {
      frame.current = f;
    }).then((fn) => (dead ? fn() : clean.push(fn)));
    subscribe<{ note: number; ledId?: string; pressed: boolean }>('input_event', (e) => {
      setHeld((previous) => {
        const s = new Set(previous);
        const key = e.ledId ?? 'key.' + e.note;
        e.pressed ? s.add(key) : s.delete(key);
        return s;
      });
    }).then((fn) => (dead ? fn() : clean.push(fn)));
    return () => {
      dead = true;
      clean.forEach((fn) => fn());
    };
  }, []);
  useEffect(() => {
    let animation = 0,
      last = 0;
    function draw(now: number) {
      animation = requestAnimationFrame(draw);
      if (now - last < 32 || document.hidden) return;
      last = now;
      const ctx = canvas.current?.getContext('2d');
      if (!ctx) return;
      const { layout, preset, paused } = data.current;
      ctx.clearRect(0, 0, layout.canvas.w, layout.canvas.h);
      const colors = native ? frame.current : frameNow();
      let accent: Color = [0, 0, 0];
      let count = 0;
      layout.leds.forEach((led, i) => {
        if (led.kind === 'none') return;
        const raw = paused ? [0, 0, 0] : (colors[i] ?? [0, 0, 0]);
        const c = raw.map((v) => Math.round(Math.pow(v / 127, 1 / preset.post.gamma) * 255));
        const color = `rgb(${c.join(',')})`;
        const { x, y } = led.pos,
          { w, h } = led.size;
        ctx.save();
        ctx.fillStyle = color;
        ctx.shadowColor = color;
        ctx.shadowBlur = 16;
        ctx.globalAlpha = 0.7;
        ctx.beginPath();
        ctx.roundRect(x, y, w, h, led.group === 'pads' ? 5 : 3);
        ctx.fill();
        ctx.shadowBlur = 0;
        ctx.globalAlpha = 1;
        ctx.fill();
        ctx.fillStyle = 'rgba(255,255,255,.08)';
        ctx.fillRect(x + 3, y + 2, w - 6, 1);
        ctx.restore();
        if (led.group === 'pads') {
          accent = accent.map((v, i) => v + c[i]) as Color;
          count++;
        }
      });
      if (Math.floor(now / 500) !== Math.floor((now - 33) / 500)) {
        const c = count
          ? accent.map((v) => Math.round(Math.min(220, Math.max(105, v / count))))
          : [157, 180, 255];
        document.documentElement.style.setProperty('--accent', `rgb(${c.join(',')})`);
      }
    }
    animation = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animation);
  }, []);
  const layout = state.layout;
  function point(e: PointerEvent) {
    const box = svg.current!.getBoundingClientRect();
    return {
      x: ((e.clientX - box.left) / box.width) * layout.canvas.w,
      y: ((e.clientY - box.top) / box.height) * layout.canvas.h,
    };
  }
  function choose(led: Led, shift: boolean) {
    onSelect(
      shift
        ? selected.includes(led.id)
          ? selected.filter((x) => x !== led.id)
          : [...selected, led.id]
        : [led.id],
    );
    if (paint) onPaint?.([led.id]);
  }
  function note(note: number, on: boolean) {
    if (state.settings.mock) onInput([on ? 144 : 128, note, on ? 100 : 0], 'keyboard');
  }
  return (
    <div className="device-wrap" style={{ transform: `scale(${zoom})` }}>
      <svg className="device-base" viewBox="0 0 1040 400" aria-hidden="true">
        <defs>
          <linearGradient id="case" x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="#30343a" />
            <stop offset="1" stopColor="#171a1e" />
          </linearGradient>
          <linearGradient id="key" x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="#838b92" />
            <stop offset=".2" stopColor="#cbd0d3" />
            <stop offset="1" stopColor="#edf0ec" />
          </linearGradient>
          <linearGradient id="blackKey" x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="#080a0d" />
            <stop offset=".9" stopColor="#262a30" />
            <stop offset="1" stopColor="#101317" />
          </linearGradient>
        </defs>
        <rect x="10" y="18" width="1020" height="372" rx="18" fill="#07090b" />
        <rect x="10" y="10" width="1020" height="374" rx="18" fill="url(#case)" stroke="#464b52" />
        <path d="M30 227H1010" stroke="#43474d" strokeWidth="1" />
        <rect x="20" y="226" width="1000" height="4" rx="2" fill="#0b0e10" />
        <text
          x="36"
          y="44"
          fill="#91999c"
          fontSize="12"
          fontFamily="Chakra Petch"
          letterSpacing="2"
        >
          61 / CONTROLLER
        </text>
        <rect
          {...{
            x: layout.decor.display.x - 4,
            y: layout.decor.display.y - 4,
            width: layout.decor.display.w + 8,
            height: layout.decor.display.h + 8,
          }}
          rx="5"
          fill="#090d10"
          stroke="#3d4248"
        />
        <rect
          {...{
            x: layout.decor.display.x,
            y: layout.decor.display.y,
            width: layout.decor.display.w,
            height: layout.decor.display.h,
          }}
          rx="2"
          fill="#091715"
        />
        <text
          x="117"
          y="80"
          textAnchor="middle"
          fill="#7dafa1"
          fontSize="9"
          fontFamily="Chakra Petch"
        >
          Keylume
        </text>
        <text
          x="117"
          y="97"
          textAnchor="middle"
          fill="#b3ddcf"
          fontSize="10"
          fontFamily="Chakra Petch"
        >
          {state.paused ? 'Paused' : state.preset.id.slice(0, 15)}
        </text>
        {layout.decor.encoders.map((e, i) => (
          <g key={i}>
            <circle cx={e.x} cy={e.y + 2} r={e.r + 2} fill="#0a0d11" />
            <circle cx={e.x} cy={e.y} r={e.r} fill="#2d3239" stroke="#555b63" />
            <path d={`M${e.x} ${e.y - e.r + 3}v7`} stroke="#a2aaad" strokeWidth="2" />
            <text x={e.x} y={e.y + e.r + 16} fill="#858c91" textAnchor="middle" fontSize="8">
              {i + 1}
            </text>
          </g>
        ))}
        {layout.decor.faders.map((f, i) => (
          <g key={i}>
            <rect x={f.x - 3} y={f.y} width="6" height={f.h} rx="3" fill="#0a0e12" />
            {[0, 1, 2, 3, 4].map((n) => (
              <path
                key={n}
                d={`M${f.x - 11} ${f.y + (n * f.h) / 4}h4`}
                stroke="#62686d"
                strokeWidth=".8"
              />
            ))}
            <rect
              x={f.x - 11}
              y={f.y + 25 + (i % 3) * 10}
              width="22"
              height="13"
              rx="3"
              fill="#40464c"
              stroke="#0d1115"
            />
            <path d={`M${f.x - 8} ${f.y + 31 + (i % 3) * 10}h16`} stroke="#888f94" />
            <text x={f.x} y="214" fill="#777f84" textAnchor="middle" fontSize="8">
              {i === 8 ? 'M' : i + 1}
            </text>
          </g>
        ))}
        {layout.decor.wheels.map((w, i) => (
          <g key={i}>
            <rect x={w.x - 3} y={w.y - 4} width={w.w + 6} height={w.h + 8} rx="8" fill="#080b10" />
            <rect x={w.x} y={w.y} width={w.w} height={w.h} rx="8" fill="#252b33" />
            {Array.from({ length: 12 }, (_, n) => (
              <path key={n} d={`M${w.x + 4} ${w.y + 10 + n * 6}h${w.w - 8}`} stroke="#464c54" />
            ))}
            <text
              x={w.x + w.w / 2}
              y={w.y + w.h + 20}
              textAnchor="middle"
              fill="#7b8389"
              fontSize="7"
            >
              {i ? 'Mod' : 'Pitch'}
            </text>
          </g>
        ))}
        {layout.decor.keys
          .filter((k) => !k.black)
          .map((k) => (
            <rect
              key={k.note}
              x={k.x}
              y="239"
              width={k.w}
              height="135"
              rx="3"
              fill={held.has('key.' + k.note) ? '#84cdb9' : 'url(#key)'}
              stroke="#10151a"
              strokeWidth=".5"
            />
          ))}
        {layout.decor.keys
          .filter((k) => k.black)
          .map((k) => (
            <rect
              key={k.note}
              x={k.x}
              y="239"
              width={k.w}
              height="84"
              rx="2"
              fill={held.has('key.' + k.note) ? '#448577' : 'url(#blackKey)'}
              stroke="#05070b"
            />
          ))}
        {layout.leds.map((l) => (
          <rect
            key={l.id}
            x={l.pos.x - 2}
            y={l.pos.y - 2}
            width={l.size.w + 4}
            height={l.size.h + 4}
            rx="5"
            fill="#0d1014"
            stroke="#42474e"
          />
        ))}
        <text x="852" y="55" fill="#bbc2c5" fontSize="8">
          ▶
        </text>
        <text x="889" y="55" fill="#bbc2c5" fontSize="8">
          ■
        </text>
      </svg>
      <canvas
        ref={canvas}
        width={layout.canvas.w}
        height={layout.canvas.h}
        className="device-glow"
      />
      <svg
        ref={svg}
        className="device-interaction"
        viewBox="0 0 1040 400"
        aria-label="Launchkey MK4 61 の LED エディター"
        onPointerDown={(e) => {
          if (e.target !== e.currentTarget) return;
          const p = point(e);
          drag.current = { ...p, ids: e.shiftKey ? selected : [] };
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const p = point(e);
          if (paint && e.buttons === 1) {
            const led = layout.leds.find(
              (l) =>
                p.x >= l.pos.x &&
                p.x <= l.pos.x + l.size.w &&
                p.y >= l.pos.y &&
                p.y <= l.pos.y + l.size.h,
            );
            if (led) onPaint?.([led.id]);
          }
          if (!drag.current) return;
          setRect({
            x: Math.min(p.x, drag.current.x),
            y: Math.min(p.y, drag.current.y),
            w: Math.abs(p.x - drag.current.x),
            h: Math.abs(p.y - drag.current.y),
          });
        }}
        onPointerUp={() => {
          if (rect && drag.current) {
            const ids = layout.leds
              .filter(
                (l) =>
                  l.pos.x + l.size.w >= rect.x &&
                  l.pos.x <= rect.x + rect.w &&
                  l.pos.y + l.size.h >= rect.y &&
                  l.pos.y <= rect.y + rect.h,
              )
              .map((l) => l.id);
            onSelect([...new Set([...drag.current.ids, ...ids])]);
          }
          drag.current = null;
          setRect(null);
        }}
      >
        {layout.decor.keys
          .filter((k) => !k.black)
          .concat(layout.decor.keys.filter((k) => k.black))
          .map((k) => (
            <rect
              key={k.note}
              x={k.x}
              y="239"
              width={k.w}
              height={k.black ? 84 : 135}
              fill="transparent"
              role="button"
              tabIndex={state.settings.mock ? 0 : -1}
              aria-label={`鍵盤 ${k.note}（画面上のみ）`}
              onPointerDown={(e) => {
                e.stopPropagation();
                e.currentTarget.setPointerCapture(e.pointerId);
                note(k.note, true);
              }}
              onPointerUp={() => note(k.note, false)}
              onPointerCancel={() => note(k.note, false)}
              onKeyDown={(e) => {
                if ((e.key === 'Enter' || e.key === ' ') && !e.repeat) {
                  e.preventDefault();
                  note(k.note, true);
                }
              }}
              onKeyUp={() => note(k.note, false)}
              onBlur={() => note(k.note, false)}
            />
          ))}
        {layout.leds.map((l) => (
          <g key={l.id}>
            <rect
              x={l.pos.x - 3}
              y={l.pos.y - 3}
              width={l.size.w + 6}
              height={l.size.h + 6}
              rx="7"
              fill="transparent"
              stroke={selected.includes(l.id) ? '#fff' : held.has(l.id) ? '#a8fff0' : 'transparent'}
              strokeWidth="1.8"
              strokeDasharray={l.kind === 'none' ? '3 2' : undefined}
              tabIndex={0}
              role="button"
              aria-label={`${l.id} ${l.kind} LED`}
              aria-pressed={selected.includes(l.id)}
              onPointerDown={(e) => {
                e.stopPropagation();
                choose(l, e.shiftKey);
                e.currentTarget.setPointerCapture(e.pointerId);
                if (state.settings.mock && l.group === 'pads' && !paint)
                  onInput([144, l.address.dawNote!, 100], 'daw');
              }}
              onPointerUp={() => {
                if (state.settings.mock && l.group === 'pads')
                  onInput([128, l.address.dawNote!, 0], 'daw');
              }}
              onPointerCancel={() => {
                if (state.settings.mock && l.group === 'pads')
                  onInput([128, l.address.dawNote!, 0], 'daw');
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  choose(l, e.shiftKey);
                }
              }}
            >
              <title>
                {l.id} · {l.kind} · 0x
                {(l.address.sysexId ?? l.address.cc ?? 0).toString(16).toUpperCase()} ·{' '}
                {l.verified ? '検証済み' : '実機未検証'}
              </title>
            </rect>
          </g>
        ))}
        {rect && (
          <rect
            x={rect.x}
            y={rect.y}
            width={rect.w}
            height={rect.h}
            fill="#7ee9c322"
            stroke="#7ee9c3"
            strokeDasharray="4 3"
            pointerEvents="none"
          />
        )}
      </svg>
    </div>
  );
}
