import type { StageSettings } from './types';
export default function HandControls({
  settings: s,
  change,
  compact = false,
}: {
  settings: StageSettings;
  change: (patch: Partial<StageSettings>) => void;
  compact?: boolean;
}) {
  return (
    <div className="stage-hand-controls">
      <label>
        練習する手{' '}
        <select
          aria-label="練習する手"
          value={s.practiceHand}
          onChange={(e) =>
            change({ practiceHand: e.target.value as StageSettings['practiceHand'] })
          }
        >
          <option value="both">両手</option>
          <option value="right">右手</option>
          <option value="left">左手</option>
        </select>
      </label>
      {!compact && (
        <>
          <label>
            手の分け方{' '}
            <select
              aria-label="手の分け方"
              value={s.handStrategy}
              onChange={(e) =>
                change({ handStrategy: e.target.value as StageSettings['handStrategy'] })
              }
            >
              <option value="auto">自動（トラック名・音域・音のつながり）</option>
              <option value="split">指定した音で左右に分割</option>
            </select>
          </label>
          <label>
            分割する音（MIDI番号）{' '}
            <input
              aria-label="手の分割音"
              type="number"
              min={0}
              max={127}
              value={s.splitPitch}
              onChange={(e) => change({ splitPitch: Number(e.target.value) })}
            />
          </label>
          <label>
            左手の色{' '}
            <input
              aria-label="左手の色"
              type="color"
              value={s.leftColor}
              onChange={(e) => change({ leftColor: e.target.value })}
            />
          </label>
          <label>
            右手の色{' '}
            <input
              aria-label="右手の色"
              type="color"
              value={s.rightColor}
              onChange={(e) => change({ rightColor: e.target.value })}
            />
          </label>
          <small>
            手の交差はMIDIだけでは確定できません。合わないときは分割音か下のトラック指定を変更してください。練習しない手はグレーです。
          </small>
        </>
      )}
    </div>
  );
}
