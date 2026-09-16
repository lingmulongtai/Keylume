import { useEffect, useRef, useState } from 'react';
import type { PointerEvent } from 'react';
import type { AppState, Color, Led } from './types';
import { native, frameNow, subscribe } from './api';
import { hardwareColor } from './input';
import { useInput } from './useInput';
import HardwareFace, { labelSize } from './HardwareFace';
import { buttonLabel, keybed } from './layout';
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
  const screenKeys = useRef(new Set<number>());
  const inputHandler = useRef(onInput);
  inputHandler.current = onInput;
  useEffect(() => {
    const release = () => {
      for (const key of screenKeys.current) inputHandler.current([128, key, 0], 'screen');
      screenKeys.current.clear();
    };
    window.addEventListener('blur', release);
    return () => {
      window.removeEventListener('blur', release);
      release();
    };
  }, []);
  const input = useInput();
  const held = new Set(input.held);
  const [rect, setRect] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  data.current = state;
  useEffect(() => {
    let dead = false;
    const clean: (() => void)[] = [];
    subscribe<Color[]>('frame_preview', (f) => {
      frame.current = f;
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
      layout.leds.forEach((led, i) => {
        if (led.kind === 'none') return;
        const raw = hardwareColor(led.id, paused ? [0, 0, 0] : (colors[i] ?? [0, 0, 0]));
        const c = raw.map((v) => Math.round(Math.pow(v / 127, 1 / preset.post.gamma) * 255));
        const color = `rgb(${c.join(',')})`;
        const { x, y } = led.pos,
          { w, h } = led.size;
        ctx.save();
        ctx.fillStyle = color;
        ctx.shadowColor = color;
        ctx.shadowBlur = 1;
        if (led.group === 'pads') {
          ctx.fillRect(x + 0.6, y + 0.6, w - 1.2, h - 1.2);
          ctx.fillStyle = 'rgba(255,255,255,.11)';
          ctx.fillRect(x + 1, y + 1, w - 2, 0.7);
        } else if (led.group === 'faderButtons') {
          ctx.fillRect(x + 5, y + h / 2 - 0.9, w - 10, 1.8);
        } else {
          ctx.font = `${labelSize(led)}px Arial`;
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(led.label ?? buttonLabel(led.id), x + w / 2, y + h / 2);
        }
        ctx.restore();
      });
    }
    animation = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animation);
  }, []);
  const layout = state.layout;
  const bed = keybed(layout);
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
    if (on === screenKeys.current.has(note)) return;
    if (on) screenKeys.current.add(note);
    else screenKeys.current.delete(note);
    onInput([on ? 144 : 128, note, on ? 100 : 0], 'screen');
  }
  return (
    <div className="device-viewport">
      <div className="input-readout" aria-label="受信中の操作">
        <span>INPUT</span>
        <span>
          {input.lastNote
            ? `Note ${input.lastNote[0]} · Velocity ${input.lastNote[1]}`
            : '鍵盤待機'}
        </span>
        <span data-testid="sustain-state">
          Sustain {input.sustain == null ? '—' : input.sustain >= 64 ? 'ON' : 'OFF'}
        </span>
        <span>
          Pressure {input.pressure ?? '—'} · Poly{' '}
          {Object.values(input.polyPressure).length
            ? Math.max(...Object.values(input.polyPressure))
            : '—'}
        </span>
        <code>{input.lastMessage || '操作すると現在値を表示します'}</code>
      </div>
      <div
        className="device-wrap"
        style={{ width: `${zoom * 100}%`, aspectRatio: `${layout.canvas.w} / ${layout.canvas.h}` }}
      >
        <svg
          className="device-base"
          viewBox={`0 0 ${layout.canvas.w} ${layout.canvas.h}`}
          aria-hidden="true"
        >
          <HardwareFace
            layout={layout}
            held={held}
            input={input}
            screen={state.paused ? 'Paused' : state.preset.id}
          />
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
          viewBox={`0 0 ${layout.canvas.w} ${layout.canvas.h}`}
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
                y={bed.y}
                width={k.w}
                height={k.black ? bed.blackHeight : bed.h}
                fill="transparent"
                role="button"
                tabIndex={0}
                aria-label={`鍵盤 ${k.note}（ピアノ）`}
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
                x={l.pos.x - 0.4}
                y={l.pos.y - 0.4}
                width={l.size.w + 0.8}
                height={l.size.h + 0.8}
                rx="1.7"
                fill="transparent"
                stroke={
                  selected.includes(l.id) ? '#fff' : held.has(l.id) ? '#a8fff0' : 'transparent'
                }
                strokeWidth={1.1 + ((input.polyPressure[l.id] ?? 0) / 127) * 2}
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
                  {l.label ?? l.id} · {l.kind} · 0x
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
    </div>
  );
}
