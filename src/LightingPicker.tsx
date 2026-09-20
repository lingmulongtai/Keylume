import { useState } from 'react';
import { ChevronDown, LayoutGrid } from 'lucide-react';
import type { ViewProps } from './ui-state';
import { Modal } from './components';
import { Miniature, PresetsScreen } from './Screens';
export default function LightingPicker(props: ViewProps) {
  const [expanded, setExpanded] = useState(false),
    [library, setLibrary] = useState(false);
  const { state, act } = props;
  return (
    <div className="lighting-picker">
      <button
        className="lighting-picker-current"
        aria-label="適用するプリセット"
        aria-expanded={expanded}
        onClick={() => setExpanded(!expanded)}
      >
        <Miniature preset={state.preset} layout={state.layout} />
        <span>{state.preset.name}</span>
        <ChevronDown size={14} />
      </button>
      {expanded && (
        <div className="lighting-picker-tray">
          <div className="lighting-picker-heading">
            <span>ライティングを選ぶ</span>
            <button onClick={() => setLibrary(true)}>
              <LayoutGrid size={13} />
              一覧・管理
            </button>
          </div>
          <div className="lighting-picker-grid">
            {state.presets.map((p) => (
              <button
                key={p.id}
                aria-label={`ライティング ${p.name}`}
                aria-pressed={p.id === state.preset.id}
                onClick={() => void act('apply_preset', { id: p.id })}
              >
                <Miniature preset={p} layout={state.layout} />
                <span>{p.name}</span>
              </button>
            ))}
          </div>
        </div>
      )}
      {library && (
        <Modal title="ライティングプリセット" wide onClose={() => setLibrary(false)}>
          <PresetsScreen {...props} />
        </Modal>
      )}
    </div>
  );
}
