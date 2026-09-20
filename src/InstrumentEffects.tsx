import type { InstrumentFx } from './types';
import { defaultInstrumentFx } from './types';
export const effectControls = [
  ['reverb', 'Reverb', '空間の響き'],
  ['delay', 'Delay', '繰り返す残響'],
  ['cutoff', 'Cutoff', '音の明るさ'],
  ['resonance', 'Resonance', 'フィルターの強調'],
  ['chorus', 'Chorus', '揺らぎと厚み'],
  ['drive', 'Drive', '歪み'],
  ['width', 'Width', 'ステレオの広がり'],
  ['tremolo', 'Tremolo', '音量の揺らぎ'],
] as const;
export default function InstrumentEffects({
  value,
  change,
}: {
  value: InstrumentFx;
  change: (value: InstrumentFx) => void;
}) {
  return (
    <details className="instrument-effects">
      <summary>音作り · 8ノブ</summary>
      <div className="effect-knobs">
        {effectControls.map(([id, name, description], i) => (
          <label key={id} className="effect-knob">
            <span>
              {i + 1} · {name}
            </span>
            <i aria-hidden="true" style={{ transform: `rotate(${-135 + value[id] * 270}deg)` }} />
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              aria-label={description}
              value={value[id]}
              onChange={(e) => change({ ...value, [id]: Number(e.target.value) })}
            />
            <output>{Math.round(value[id] * 100)}%</output>
            <small>{description}</small>
          </label>
        ))}
      </div>
      <button onClick={() => change({ ...defaultInstrumentFx })}>エフェクトをリセット</button>
      <small>本体ノブの初期割当です。割当画面で変更できます。</small>
    </details>
  );
}
