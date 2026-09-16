import { memo, useId } from 'react';
import type { Color, Layout, Led } from './types';
import { hardwareColor, type InputState } from './input';
import { buttonLabel, keybed } from './layout';

export const labelSize = (led: Led) =>
  ['‹', '›', '⌃', '⌄', '▶', '■', '●', '↻'].includes(led.label ?? buttonLabel(led.id))
    ? 7
    : Math.min(3.8, led.size.w / ((led.label ?? buttonLabel(led.id)).length * 0.55));
export const ledColor = (color: Color, gamma: number) =>
  `rgb(${color.map((v) => Math.round(Math.pow(v / 127, 1 / gamma) * 255)).join(',')})`;

function HardwareFace({
  layout,
  held,
  input,
  screen = 'Keylume',
  colors,
  gamma = 2.2,
}: {
  layout: Layout;
  held?: Set<string>;
  input?: InputState;
  screen?: string;
  colors?: Color[];
  gamma?: number;
}) {
  const id = useId();
  const bed = keybed(layout);
  const { w, h } = layout.canvas;
  const display = layout.decor.display;
  const text = { fill: '#aaaaad', fontFamily: 'Arial, sans-serif', fontSize: 3.8 };
  return (
    <>
      <defs>
        <linearGradient id={`${id}-white`} x2="0" y2="1">
          <stop stopColor="#b8b8ba" />
          <stop offset=".14" stopColor="#e3e3e4" />
          <stop offset="1" stopColor="#f0f0f0" />
        </linearGradient>
        <linearGradient id={`${id}-black`} x2="0" y2="1">
          <stop stopColor="#080809" />
          <stop offset=".86" stopColor="#19191c" />
          <stop offset="1" stopColor="#303033" />
        </linearGradient>
      </defs>
      <rect
        data-control="chassis"
        x="1"
        y="1"
        width={w - 2}
        height={h - 2}
        rx="4"
        fill="#262629"
        stroke="#4a4a4d"
        strokeWidth="1"
      />
      <path d={`M5 ${h - 6}H${w - 5}`} stroke="#09090b" strokeWidth="5" />
      <rect x={w * 0.02} y={bed.y - 2} width={w * 0.96} height={bed.h + 5} fill="#08080a" />
      {layout.decor.keys
        .filter((k) => !k.black)
        .map((k) => (
          <rect
            key={k.note}
            data-key={k.note}
            x={k.x}
            y={bed.y}
            width={k.w}
            height={bed.h}
            rx="1.7"
            stroke="#535356"
            strokeWidth=".5"
            fill={held?.has('key.' + k.note) ? '#e8c48e' : `url(#${id}-white)`}
          />
        ))}
      {layout.decor.keys
        .filter((k) => k.black)
        .map((k) => (
          <g key={k.note} data-key={k.note}>
            <rect
              x={k.x - 0.6}
              y={bed.y}
              width={k.w + 1.2}
              height={bed.blackHeight + 2}
              rx="1"
              fill="#09090b"
            />
            <rect
              x={k.x}
              y={bed.y}
              width={k.w}
              height={bed.blackHeight}
              rx="1"
              fill={held?.has('key.' + k.note) ? '#b99359' : `url(#${id}-black)`}
            />
            <path
              d={`M${k.x + 2} ${bed.y + 5}v${bed.blackHeight - 12}`}
              stroke="#55555a"
              strokeWidth=".65"
            />
          </g>
        ))}
      <text
        x={w * 0.023}
        y={bed.y - 13}
        fill="#c0c0c3"
        fontSize="7.8"
        fontFamily="Arial, sans-serif"
        fontWeight="600"
      >
        LAUNCHKEY
      </text>
      {layout.decor.wheels.map((wheel, i) => (
        <g
          key={i}
          data-control={i ? 'mod-wheel' : 'pitch-wheel'}
          data-value={(i ? input?.modulation : input?.pitch) ?? 'unknown'}
        >
          <title>
            {i ? 'Modulation' : 'Pitch'}: {(i ? input?.modulation : input?.pitch) ?? '未取得'}
          </title>
          <rect
            x={wheel.x - 1.7}
            y={wheel.y - 1.7}
            width={wheel.w + 3.4}
            height={wheel.h + 3.4}
            rx="1.5"
            fill="#0b0b0d"
          />
          <rect
            x={wheel.x}
            y={wheel.y}
            width={wheel.w}
            height={wheel.h}
            rx="3"
            fill="#19191c"
            stroke="#424246"
            strokeWidth=".6"
          />
          {Array.from({ length: 12 }, (_, n) => (
            <path
              key={n}
              d={`M${wheel.x + 2} ${wheel.y + 4 + (n * (wheel.h - 8)) / 11}h${wheel.w - 4}`}
              stroke={n === 5 ? '#55555a' : '#303034'}
            />
          ))}
          {(i ? input?.modulation : input?.pitch) != null && (
            <path
              d={`M${wheel.x + 1} ${wheel.y + 4 + (1 - (i ? input!.modulation! / 127 : input!.pitch! / 16383)) * (wheel.h - 8)}h${wheel.w - 2}`}
              stroke="#e8c48e"
              strokeWidth="2"
            />
          )}
          <text {...text} x={wheel.x + wheel.w / 2} y={wheel.y + wheel.h + 8} textAnchor="middle">
            {i ? 'Modulation' : 'Pitch'}
          </text>
        </g>
      ))}
      {layout.decor.faders.map((fader, i) => (
        <g
          key={i}
          data-control={`fader-${i + 1}`}
          data-value={input?.faders[i] ?? 'unknown'}
          opacity={input && input.faders[i] == null ? 0.55 : 1}
        >
          <title>
            Fader {i + 1}: {input?.faders[i] ?? '未取得'}
          </title>
          <rect
            x={fader.x - 1.7}
            y={fader.y}
            width="3.4"
            height={fader.h}
            rx="1.7"
            fill="#09090b"
            stroke="#3c3c40"
            strokeWidth=".7"
          />
          {Array.from({ length: 9 }, (_, n) => (
            <path
              key={n}
              d={`M${fader.x + 10} ${fader.y + 4 + (n * (fader.h - 8)) / 8}h5`}
              stroke="#909094"
              strokeWidth=".45"
            />
          ))}
          <rect
            x={fader.x - 7.7}
            y={fader.y + (fader.h - 6.5) * (1 - (input?.faders[i] ?? 64) / 127)}
            width="15.4"
            height="6.5"
            rx="1.2"
            fill="#101013"
            stroke="#505056"
            strokeWidth=".7"
          />
          <path
            d={`M${fader.x - 5.4} ${fader.y + (fader.h - 6.5) * (1 - (input?.faders[i] ?? 64) / 127) + 1.8}h10.8`}
            stroke="#8d8d93"
            strokeWidth="1"
          />
        </g>
      ))}
      <rect
        data-control="oled"
        x={display.x - 1}
        y={display.y - 1}
        width={display.w + 2}
        height={display.h + 2}
        rx="1"
        fill="#101012"
        stroke="#4c4c51"
        strokeWidth=".8"
      />
      <rect
        x={display.x + 4}
        y={display.y + 3}
        width={display.w - 8}
        height={display.h - 6}
        fill="#09090b"
      />
      <text
        x={display.x + display.w / 2}
        y={display.y + display.h / 2 + 2}
        textAnchor="middle"
        fill="#c8c8cb"
        fontSize="4"
        fontFamily="Arial, sans-serif"
      >
        {screen.slice(0, 16)}
      </text>
      {layout.decor.encoders.map((encoder, i) => (
        <g
          key={i}
          data-control={`encoder-${i + 1}`}
          data-value={input?.encoders[i] ?? 'unknown'}
          data-relative={input?.relative[i]}
          opacity={input && input.encoders[i] == null ? 0.55 : 1}
        >
          <title>
            Encoder {i + 1}:{' '}
            {input?.encoders[i] == null
              ? '未取得'
              : input.relative[i]
                ? '相対回転'
                : input.encoders[i]}
          </title>
          <circle cx={encoder.x} cy={encoder.y + 1.6} r={encoder.r + 0.6} fill="#0e0e11" />
          <circle
            cx={encoder.x}
            cy={encoder.y}
            r={encoder.r}
            fill="#333337"
            stroke="#525257"
            strokeWidth=".7"
          />
          <path
            d={`M${encoder.x} ${encoder.y - encoder.r + 1.3}v2.4`}
            transform={`rotate(${input?.encoders[i] == null ? 0 : input.relative[i] ? (input.encoders[i]! / 128) * 360 : (input.encoders[i]! / 127) * 270 - 135} ${encoder.x} ${encoder.y})`}
            stroke="#99999d"
            strokeWidth=".8"
          />
        </g>
      ))}
      {layout.decor.controls?.map((control) => (
        <g key={control.id} data-control={control.id}>
          <rect
            x={control.pos.x}
            y={control.pos.y}
            width={control.size.w}
            height={control.size.h}
            rx="1.4"
            fill={held?.has(control.id) ? '#73634b' : '#1b1b1e'}
            stroke="#0d0d10"
            strokeWidth=".8"
          />
          <text
            {...text}
            x={control.pos.x + control.size.w / 2}
            y={control.pos.y + control.size.h / 2 + 1.2}
            textAnchor="middle"
          >
            {control.label}
          </text>
        </g>
      ))}
      {layout.leds.map((led, i) => (
        <g key={led.id} data-control={led.id}>
          <rect
            x={led.pos.x}
            y={led.pos.y}
            width={led.size.w}
            height={led.size.h}
            rx={led.group === 'pads' ? 0.8 : 1.3}
            fill={
              led.group === 'pads'
                ? colors
                  ? ledColor(hardwareColor(led.id, colors[i] ?? [0, 0, 0]), gamma)
                  : '#505057'
                : held?.has(led.id)
                  ? '#73634b'
                  : '#1a1a1d'
            }
            stroke="#101013"
            strokeWidth="1"
          />
          {led.group === 'pads' ? (
            <path
              d={`M${led.pos.x + 1} ${led.pos.y + led.size.h - 1}h${led.size.w - 2}`}
              stroke="#ffffff24"
              strokeWidth=".7"
            />
          ) : led.group === 'faderButtons' ? (
            <rect
              x={led.pos.x + 5}
              y={led.pos.y + led.size.h / 2 - 0.9}
              width={led.size.w - 10}
              height="1.8"
              rx=".4"
              fill={
                colors ? ledColor(hardwareColor(led.id, colors[i] ?? [0, 0, 0]), gamma) : '#898993'
              }
            />
          ) : (
            <text
              x={led.pos.x + led.size.w / 2}
              y={led.pos.y + led.size.h / 2}
              textAnchor="middle"
              dominantBaseline="central"
              fontFamily="Arial, sans-serif"
              fontSize={labelSize(led)}
              fill={
                colors
                  ? ledColor(hardwareColor(led.id, colors[i] ?? [0, 0, 0]), gamma)
                  : led.id === 'btn.play'
                    ? '#00a050'
                    : led.id === 'btn.record'
                      ? '#c43c38'
                      : '#9a9aa1'
              }
            >
              {led.label ?? buttonLabel(led.id)}
            </text>
          )}
        </g>
      ))}
      {layout.geometryRevision === 2 && (
        <g {...text} fontSize="3.6">
          {[
            'Volume',
            'Custom 1',
            'Custom 2',
            'Custom 3',
            'Custom 4',
            'Part A',
            'Part B',
            'Split',
            'Layer',
          ].map((label, i) => (
            <text key={label} x={233.4 + 26.9 * i} y="115.5" textAnchor="middle">
              {label}
            </text>
          ))}
          {[
            'Plugin',
            'Mixer',
            'Sends',
            'Transport',
            'Custom 1',
            'Custom 2',
            'Custom 3',
            'Custom 4',
          ].map((label, i) => (
            <text key={label} x={570 + 26.9 * i} y="56.8" textAnchor="middle">
              {label}
            </text>
          ))}
          {[
            'DAW',
            'Drum',
            'User Chord',
            'Arp Pattern',
            'Custom 1',
            'Custom 2',
            'Custom 3',
            'Custom 4',
          ].map((label, i) => (
            <text key={label} x={570 + 26.9 * i} y="116.4" textAnchor="middle">
              {label}
            </text>
          ))}
          <text x="199.5" y="49.6" textAnchor="middle">
            Octave
          </text>
          <text x="500" y="57.8" textAnchor="middle">
            Track
          </text>
          <text x="659" y="51" textAnchor="middle">
            Encoder Mode
          </text>
          <text x="659" y="122.4" textAnchor="middle">
            Pad Mode
          </text>
          <text x="840.7" y="39" textAnchor="middle">
            Redo
          </text>
          <text x="486.5" y="115.5" textAnchor="middle">
            Latch
          </text>
          <text x="448.6" y="99.5" textAnchor="middle" fontSize="2.8">
            Arm/Select
          </text>
        </g>
      )}
    </>
  );
}
export default memo(HardwareFace);
