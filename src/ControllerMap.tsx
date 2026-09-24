import HardwareFace from './HardwareFace';
import { useInput } from './useInput';
import { bindingName, type Binding } from './controller';
import type { Layout } from './types';
export default function ControllerMap({
  layout,
  selected,
  bindings,
  names,
  select,
  inform,
}: {
  layout: Layout;
  selected: string;
  bindings: Record<string, Binding>;
  names: Record<string, string>;
  select: (id: string) => void;
  inform: (message: string) => void;
}) {
  const input = useInput();
  const areas = [
    ...layout.leds.map((l) => ({ id: l.id, x: l.pos.x, y: l.pos.y, w: l.size.w, h: l.size.h })),
    ...layout.decor.encoders.map((k, i) => ({
      id: `encoder-${i + 1}`,
      x: k.x - k.r - 4,
      y: k.y - k.r - 4,
      w: (k.r + 4) * 2,
      h: (k.r + 4) * 2 + 8,
    })),
    ...layout.decor.faders.map((f, i) => ({
      id: `fader-${i + 1}`,
      x: f.x - 12,
      y: f.y,
      w: 24,
      h: f.h + 8,
    })),
    ...layout.decor.wheels.map((k, i) => ({
      id: i ? 'mod-wheel' : 'pitch-wheel',
      x: k.x,
      y: k.y,
      w: k.w,
      h: k.h,
    })),
  ];
  return (
    <section className="controller-map" aria-label="本体から割り当てを選ぶ">
      <p>本体のボタン・ノブ・フェーダーをクリックして割り当てを編集</p>
      <svg viewBox={`0 0 ${layout.canvas.w} ${layout.canvas.h}`}>
        <HardwareFace layout={layout} input={input} held={new Set(input.held)} />
        {areas.map((a) => (
          <g
            key={a.id}
            role="button"
            tabIndex={0}
            aria-label={`割り当て: ${names[a.id] ?? a.id}`}
            aria-pressed={selected === a.id}
            onClick={() => select(a.id)}
            onKeyDown={(e) => {
              if (['Enter', ' '].includes(e.key)) {
                e.preventDefault();
                select(a.id);
              }
            }}
          >
            <title>
              {names[a.id] ?? a.id} · {bindingName(bindings[a.id])}
            </title>
            <rect
              x={a.x - 1}
              y={a.y - 1}
              width={a.w + 2}
              height={a.h + 2}
              rx="2"
              fill={selected === a.id ? '#e8c48e26' : 'transparent'}
              stroke={selected === a.id ? '#e8c48e' : 'transparent'}
              strokeWidth="1.4"
            />
          </g>
        ))}
        {layout.decor.controls?.map((c) => (
          <g
            key={c.id}
            role="button"
            tabIndex={0}
            aria-label={`本体設定: ${c.label}`}
            onClick={() =>
              inform(
                `${c.label} は本体側の機能です。信号を送る設定では「本体を操作して選択」で割り当てられます。`,
              )
            }
            onKeyDown={(e) => {
              if (e.key === 'Enter')
                inform(`${c.label} は本体側の機能です。MIDI Learnで受信を確認できます。`);
            }}
          >
            <title>{c.label} · 本体の機能</title>
            <rect x={c.pos.x} y={c.pos.y} width={c.size.w} height={c.size.h} fill="transparent" />
          </g>
        ))}
      </svg>
      <small>{input.lastMessage || '本体を操作すると押下と現在値を表示します'}</small>
    </section>
  );
}
