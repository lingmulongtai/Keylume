import type { StageSettings } from './types';
export default function PlaybackControls({
  settings: s,
  change,
}: {
  settings: StageSettings;
  change: (patch: Partial<StageSettings>) => void;
}) {
  return (
    <div className="stage-hand-controls">
      <label>
        進め方{' '}
        <select
          aria-label="練習の進め方"
          value={s.practiceMode}
          onChange={(e) =>
            change({ practiceMode: e.target.value as StageSettings['practiceMode'] })
          }
        >
          <option value="timing">テンポで進む・採点</option>
          <option value="wait">正しい音まで待つ</option>
          <option value="listen">試聴（採点なし）</option>
        </select>
      </label>
      <label>
        MIDIの音{' '}
        <select
          aria-label="MIDIの音"
          value={s.playbackMode}
          onChange={(e) =>
            change({ playbackMode: e.target.value as StageSettings['playbackMode'] })
          }
        >
          <option value="off">オフ</option>
          <option value="accompaniment">練習しない手を伴奏（試聴時は全部）</option>
          <option value="full">全体を自動演奏</option>
        </select>
      </label>
      <label>
        再生音量{' '}
        <input
          aria-label="MIDI再生音量"
          type="range"
          min={0}
          max={1}
          step={0.01}
          value={s.playbackVolume}
          onChange={(e) => change({ playbackVolume: Number(e.target.value) })}
        />
        <output>{Math.round(s.playbackVolume * 100)}%</output>
      </label>
      <label>
        音のタイミング（ms）{' '}
        <input
          aria-label="MIDI音声タイミング"
          type="number"
          min={-1000}
          max={1000}
          step={10}
          value={s.audioOffsetMs}
          onChange={(e) => change({ audioOffsetMs: Number(e.target.value) })}
        />
      </label>
    </div>
  );
}
