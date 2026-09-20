import { useState } from 'react';
import type { ViewProps } from '../ui-state';
import { Slider } from '../components';
import { drumNames, kitNames, defaultDrumKit, type PadSound } from '../drum-kits';
export default function DrumEditor({
  state,
  saveSettings,
}: Pick<ViewProps, 'state' | 'saveSettings'>) {
  const [pad, setPad] = useState(8);
  const piano = state.settings.piano,
    kit = piano.drumKit;
  const sound = kit.banks[kit.kit][pad];
  const update = (patch: Partial<PadSound>) =>
    saveSettings({
      ...state.settings,
      piano: {
        ...piano,
        drumKit: {
          ...kit,
          banks: kit.banks.map((bank, i) =>
            i === kit.kit ? bank.map((p, j) => (j === pad ? { ...p, ...patch } : p)) : bank,
          ),
        },
      },
    });
  return (
    <details className="drum-editor">
      <summary>パッドの音作り</summary>
      <div className="drum-editor-fields">
        <label className="field">
          <span>編集するパッド</span>
          <select
            aria-label="編集するパッド"
            value={pad}
            onChange={(e) => setPad(Number(e.target.value))}
          >
            {drumNames.map((_, i) => (
              <option key={i} value={i}>
                Pad {i + 1} · {drumNames[kit.banks[kit.kit][i].kind]}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>音の種類</span>
          <select
            aria-label="パッドの音の種類"
            value={sound.kind}
            onChange={(e) => update({ kind: Number(e.target.value) })}
          >
            {drumNames.map((name, i) => (
              <option key={name} value={i}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <Slider
          label="パッドの音程"
          min={-24}
          max={24}
          step={1}
          value={sound.tune}
          display={`${sound.tune > 0 ? '+' : ''}${sound.tune} st`}
          onChange={(tune) => update({ tune })}
        />
        <Slider
          label="パッドの余韻"
          min={0.15}
          max={4}
          step={0.05}
          value={sound.decay}
          display={`${sound.decay.toFixed(2)} ×`}
          onChange={(decay) => update({ decay })}
        />
        <Slider label="パッドの音量" value={sound.level} onChange={(level) => update({ level })} />
        <Slider
          label="パッドの定位"
          min={-1}
          max={1}
          step={0.01}
          value={sound.pan}
          display={
            sound.pan === 0
              ? '中央'
              : `${sound.pan < 0 ? 'L' : 'R'} ${Math.round(Math.abs(sound.pan) * 100)}`
          }
          onChange={(pan) => update({ pan })}
        />
      </div>
      <button
        onClick={() =>
          saveSettings({
            ...state.settings,
            piano: {
              ...piano,
              drumKit: {
                ...kit,
                banks: kit.banks.map((bank, i) =>
                  i === kit.kit ? defaultDrumKit().banks[i] : bank,
                ),
              },
            },
          })
        }
      >
        {kitNames[kit.kit]} を初期状態に戻す
      </button>
      <p className="stage-hint">
        調整はキットごとに保存します。本体パッド左の上下ボタンでキットを切り替えます。
      </p>
    </details>
  );
}
