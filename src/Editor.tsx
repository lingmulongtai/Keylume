import { useState } from 'react';
import {
  Plus,
  Eye,
  EyeOff,
  Trash2,
  ArrowUp,
  ArrowDown,
  MousePointer2,
  Paintbrush,
  Undo2,
  Layers3,
  Save,
  Minus,
  Maximize2,
} from 'lucide-react';
import type { ViewProps } from './ui-state';
import type { Layer, Zone, Blend } from './types';
import { effects, zones, blends, uid, includesLed } from './types';
import Piano from './Piano';
import { command } from './api';
import DeviceCanvas from './DeviceCanvas';
import { Slider, Note, Modal } from './components';
export default function Editor({
  state,
  act,
  edit,
  toast,
  onSave,
  saveSettings,
}: ViewProps & { onSave: () => void }) {
  const [selected, setSelected] = useState<string[]>([]),
    [selectedLayer, setSelectedLayer] = useState(state.preset.layers[0]?.id ?? ''),
    [paint, setPaint] = useState(false),
    [brush, setBrush] = useState('#74fbd4'),
    [zoom, setZoom] = useState(1),
    [adding, setAdding] = useState(false);
  const layer = state.preset.layers.find((l) => l.id === selectedLayer) ?? state.preset.layers[0];
  const [history, setHistory] = useState<(typeof state.preset)[]>([]);
  const change = (next: typeof state.preset) => {
    setHistory((h) => [...h.slice(-19), structuredClone(state.preset)]);
    edit(next);
  };
  const patch = (data: Partial<Layer>) => {
    if (layer)
      change({
        ...state.preset,
        layers: state.preset.layers.map((l) => (l.id === layer.id ? { ...l, ...data } : l)),
      });
  };
  const parameter = (key: string, value: unknown) =>
    patch({ params: { ...layer.params, [key]: value } });
  const selectZone = (zone: Zone) => {
    setSelected(state.layout.leds.filter((l) => includesLed(zone, l)).map((l) => l.id));
    patch({ zone });
  };
  const paintLeds = (ids: string[]) => {
    let p = state.preset;
    let target = p.layers.find((l) => l.effect === 'paint');
    if (!target) {
      target = {
        id: uid(),
        effect: 'paint',
        params: { colorsByLed: {} },
        zone: 'all',
        opacity: 1,
        blend: 'normal',
        enabled: true,
      };
      p = { ...p, layers: [target, ...p.layers] };
    }
    const map = { ...((target.params.colorsByLed as Record<string, string>) ?? {}) };
    if (ids.every((id) => map[id] === brush)) return;
    ids.forEach((id) => (map[id] = brush));
    const updated = { ...target, params: { ...target.params, colorsByLed: map } };
    edit({ ...p, layers: p.layers.map((l) => (l.id === target.id ? updated : l)) });
    setSelectedLayer(target.id);
  };
  const move = (id: string, d: number) => {
    const layers = [...state.preset.layers];
    const i = layers.findIndex((l) => l.id === id);
    if (i + d < 0 || i + d >= layers.length) return;
    [layers[i], layers[i + d]] = [layers[i + d], layers[i]];
    change({ ...state.preset, layers });
  };
  return (
    <div className="editor-grid">
      <section className="editor-main">
        <div className="workspace-top">
          <div>
            <h1>ライティング</h1>
          </div>
          <select
            className="preset-picker"
            aria-label="適用するプリセット"
            value={state.preset.id}
            onChange={(event) => act('apply_preset', { id: event.target.value })}
          >
            {state.presets.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {preset.name}
              </option>
            ))}
          </select>
        </div>
        <div className="device-stage">
          <div className="stage-label">
            <span>Launchkey MK4 61</span>
            <small>61鍵モデル · 上面</small>
          </div>
          <div className="stage-tools">
            <button
              className={'icon-button ' + (!paint ? 'active' : '')}
              aria-label="LED を選択"
              onClick={() => setPaint(false)}
            >
              <MousePointer2 size={16} />
            </button>
            <button
              className={'icon-button ' + (paint ? 'active' : '')}
              aria-label="ペイントモード"
              onClick={() => setPaint(!paint)}
            >
              <Paintbrush size={16} />
            </button>
            {paint && (
              <input
                aria-label="ブラシ色"
                type="color"
                value={brush}
                onChange={(e) => setBrush(e.target.value)}
              />
            )}
          </div>
          <DeviceCanvas
            state={state}
            selected={selected}
            onSelect={setSelected}
            paint={paint}
            onPaint={paintLeds}
            zoom={zoom}
            onInput={(bytes, source) => {
              void command(source === 'screen' ? 'piano_input' : 'simulate_input', {
                bytes,
                source,
              }).catch((e) => toast(String(e)));
            }}
          />
          <div className="stage-bottom">
            <span>
              <span className="tiny-dot" />{' '}
              {state.settings.mock ? 'プレビュー中' : '実機からのフレーム'}
              <i />
              {selected.length} LED 選択
            </span>
            <div>
              <button
                className="icon-button"
                aria-label="縮小"
                onClick={() => setZoom(Math.max(0.7, zoom - 0.1))}
              >
                <Minus size={14} />
              </button>
              <span className="numeric">{Math.round(zoom * 100)}%</span>
              <button
                className="icon-button"
                aria-label="拡大"
                onClick={() => setZoom(Math.min(2, zoom + 0.1))}
              >
                <Plus size={14} />
              </button>
              <button
                className="icon-button"
                aria-label="表示をリセット"
                onClick={() => setZoom(1)}
              >
                <Maximize2 size={14} />
              </button>
            </div>
          </div>
        </div>
        <Piano state={state} saveSettings={saveSettings} toast={toast} />
        <div className="zones">
          <span>ゾーン</span>
          {Object.entries(zones)
            .filter(([key]) => key !== 'pads')
            .map(([key, label]) => (
              <button
                key={key}
                className={layer?.zone === key ? 'selected' : ''}
                onClick={() => selectZone(key as Zone)}
              >
                {label
                  .replace('すべての LED', 'すべて')
                  .replace('その他の', '')
                  .replace('フェーダーボタン', 'フェーダー')}
              </button>
            ))}
          <button
            disabled={!selected.length}
            onClick={() => {
              patch({ zone: selected });
              toast(`${selected.length} 個の LED をカスタムゾーンに設定しました`);
            }}
          >
            選択 {selected.length}
          </button>
        </div>
        <div className="layers-section">
          <div className="section-heading">
            <h2>
              レイヤー <small>{state.preset.layers.length}</small>
            </h2>
            <div>
              <button
                className="icon-button"
                aria-label="編集を元に戻す"
                disabled={!history.length}
                onClick={() => {
                  edit(history[history.length - 1]);
                  setHistory(history.slice(0, -1));
                }}
              >
                <Undo2 size={16} />
              </button>
              <button className="subtle" onClick={() => setAdding(true)}>
                <Plus size={15} />
                レイヤーを追加
              </button>
            </div>
          </div>
          <p className="micro">上のレイヤーが優先されます。</p>
          {state.preset.layers.map((l, i) => (
            <div key={l.id} className={'layer-row ' + (l.id === layer?.id ? 'selected' : '')}>
              <span className="layer-number numeric">{i + 1}</span>
              <button className="layer-select" onClick={() => setSelectedLayer(l.id)}>
                <span className={'effect-icon effect-' + l.effect}>
                  <Layers3 size={17} />
                </span>
                <span>
                  <strong>{effects[l.effect]}</strong>
                  <small>
                    {Array.isArray(l.zone) ? `カスタム · ${l.zone.length} LED` : zones[l.zone]}
                  </small>
                </span>
              </button>
              <span className="blend-label">{blends[l.blend]}</span>
              <span className="numeric opacity-label">{Math.round(l.opacity * 100)}%</span>
              <button
                className="icon-button"
                aria-label={
                  l.enabled ? `${effects[l.effect]}を非表示` : `${effects[l.effect]}を表示`
                }
                onClick={() =>
                  change({
                    ...state.preset,
                    layers: state.preset.layers.map((x) =>
                      x.id === l.id ? { ...x, enabled: !x.enabled } : x,
                    ),
                  })
                }
              >
                {l.enabled ? <Eye size={16} /> : <EyeOff size={16} />}
              </button>
              <div className="order-buttons">
                <button
                  className="icon-button"
                  aria-label="レイヤーを上へ"
                  disabled={i === 0}
                  onClick={() => move(l.id, -1)}
                >
                  <ArrowUp size={12} />
                </button>
                <button
                  className="icon-button"
                  aria-label="レイヤーを下へ"
                  disabled={i === state.preset.layers.length - 1}
                  onClick={() => move(l.id, 1)}
                >
                  <ArrowDown size={12} />
                </button>
              </div>
            </div>
          ))}
          {!state.preset.layers.length && (
            <div className="empty-state">
              <Layers3 />
              <p>レイヤーを追加してライティングを作成します。</p>
            </div>
          )}
        </div>
        <div className="editor-tip">
          <div>
            クリックで選択 · Shift + クリックで追加 · 背景をドラッグして範囲選択
            <small>
              鍵盤・ノブは発光しません。
              {state.settings.mock
                ? '画面の鍵盤とパッドで入力を試せます。'
                : '操作中も実機へ反映します。'}
            </small>
          </div>
        </div>
      </section>
      <aside className="inspector">
        <div className="inspector-title">
          <span>インスペクタ</span>
          <span className="micro">ライブ編集</span>
        </div>
        {layer ? (
          <>
            <div className="inspector-effect">
              <span className="effect-icon large">
                <Layers3 size={24} />
              </span>
              <div>
                <h2>{effects[layer.effect]}</h2>
                <small>{Array.isArray(layer.zone) ? 'カスタムゾーン' : zones[layer.zone]}</small>
              </div>
              <button
                className="icon-button"
                aria-label="レイヤーを削除"
                onClick={() =>
                  change({
                    ...state.preset,
                    layers: state.preset.layers.filter((l) => l.id !== layer.id),
                  })
                }
              >
                <Trash2 size={16} />
              </button>
            </div>
            <label className="field">
              <span>エフェクト</span>
              <select value={layer.effect} onChange={(e) => patch({ effect: e.target.value })}>
                {Object.entries(effects).map(([k, v]) => (
                  <option key={k} value={k}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
            <div className="divider" />
            <h4>カラー</h4>
            <div className="color-picker">
              <input
                aria-label="エフェクトの色"
                type="color"
                value={String(layer.params.color ?? '#43ffc2')}
                onChange={(e) => parameter('color', e.target.value)}
              />
              <span className="numeric">
                {String(layer.params.color ?? '#43ffc2').toUpperCase()}
              </span>
              <span className="micro">RGB</span>
            </div>
            {['aurora', 'gradient', 'breathing'].includes(layer.effect) && (
              <div className="color-stops">
                {(Array.isArray(layer.params.colors)
                  ? layer.params.colors
                  : ['#43ffc2', '#38a7bf', '#a574f9']
                ).map((color, i, colors) => (
                  <label key={i}>
                    <input
                      aria-label={`グラデーション色 ${i + 1}`}
                      type="color"
                      value={String(color)}
                      onChange={(e) =>
                        parameter(
                          'colors',
                          colors.map((c, j) => (i === j ? e.target.value : c)),
                        )
                      }
                    />
                  </label>
                ))}
              </div>
            )}
            {!['static', 'paint', 'gradient', 'hardware_fx'].includes(layer.effect) && (
              <Slider
                label="速度"
                value={Number(layer.params.speed ?? 0.6)}
                min={0.05}
                max={3}
                step={0.05}
                display={`${Number(layer.params.speed ?? 0.6).toFixed(2)}×`}
                onChange={(v) => parameter('speed', v)}
              />
            )}
            {['breathing', 'spectrum_cycle'].includes(layer.effect) && (
              <Slider
                label="周期"
                value={Number(layer.params.period ?? 6)}
                min={0.5}
                max={20}
                step={0.5}
                display={`${layer.params.period ?? 6} 秒`}
                onChange={(v) => parameter('period', v)}
              />
            )}
            {layer.effect === 'gradient' && (
              <Slider
                label="角度"
                value={Number(layer.params.angle ?? 0)}
                min={0}
                max={360}
                step={1}
                display={`${layer.params.angle ?? 0}°`}
                onChange={(v) => parameter('angle', v)}
              />
            )}
            {['wave', 'aurora'].includes(layer.effect) && (
              <label className="field">
                <span>方向</span>
                <select
                  value={String(layer.params.direction ?? 'right')}
                  onChange={(e) => parameter('direction', e.target.value)}
                >
                  <option value="right">左から右へ →</option>
                  <option value="left">右から左へ ←</option>
                  <option value="down">上から下へ ↓</option>
                  <option value="out">中心から外へ</option>
                  <option value="in">外から中心へ</option>
                </select>
              </label>
            )}
            {layer.effect === 'wave' && (
              <Slider
                label="波長"
                min={0.1}
                max={2}
                value={Number(layer.params.wavelength ?? 0.8)}
                display={`${Number(layer.params.wavelength ?? 0.8).toFixed(2)}`}
                onChange={(v) => parameter('wavelength', v)}
              />
            )}
            {['ripple', 'reactive'].includes(layer.effect) && (
              <Slider
                label="残光時間"
                min={0.1}
                max={8}
                step={0.1}
                value={Number(layer.params.decay ?? 2)}
                display={`${layer.params.decay ?? 2} 秒`}
                onChange={(v) => parameter('decay', v)}
              />
            )}
            {layer.effect === 'starlight' && (
              <Slider
                label="星の密度"
                min={0.02}
                max={1}
                value={Number(layer.params.density ?? 0.3)}
                onChange={(v) => parameter('density', v)}
              />
            )}
            {layer.effect === 'fire' && (
              <Slider
                label="炎の強さ"
                min={0.1}
                max={2}
                value={Number(layer.params.intensity ?? 1)}
                onChange={(v) => parameter('intensity', v)}
              />
            )}
            {layer.effect.startsWith('audio_') && (
              <>
                <Slider
                  label="感度"
                  min={0.1}
                  max={4}
                  value={Number(layer.params.sensitivity ?? 1)}
                  display={`${Number(layer.params.sensitivity ?? 1).toFixed(1)}×`}
                  onChange={(v) => parameter('sensitivity', v)}
                />
                <Note>
                  {state.status.audio === 'capturing'
                    ? 'PC の再生音を解析しています。'
                    : '音声キャプチャはデスクトップ版で利用できます。取得できない場合は無音として表示します。'}
                </Note>
              </>
            )}
            {['tempo_pulse', 'metronome'].includes(layer.effect) && (
              <>
                <Slider
                  label="テンポ"
                  min={30}
                  max={240}
                  step={1}
                  value={Number(layer.params.bpm ?? 120)}
                  display={`${layer.params.bpm ?? 120} BPM`}
                  onChange={(v) => parameter('bpm', v)}
                />
                <label className="field">
                  <span>BPM ソース</span>
                  <select
                    value={String(layer.params.source ?? 'fixed')}
                    onChange={(e) => parameter('source', e.target.value)}
                  >
                    <option value="fixed">固定</option>
                    <option value="midi">MIDI クロック / タップ</option>
                  </select>
                </label>
                <button className="subtle" onClick={() => void act('tap_tempo')}>
                  タップテンポ · {Math.round(state.status.bpm)}
                </button>
              </>
            )}
            {layer.effect === 'hardware_fx' && (
              <>
                <Slider
                  label="パレット番号"
                  min={0}
                  max={127}
                  step={1}
                  value={Number(layer.params.palette ?? 76)}
                  display={String(layer.params.palette ?? 76)}
                  onChange={(v) => parameter('palette', v)}
                />
                <label className="field">
                  <span>本体エフェクト</span>
                  <select
                    value={String(layer.params.mode ?? 'pulse')}
                    onChange={(e) => parameter('mode', e.target.value)}
                  >
                    <option value="static">静止</option>
                    <option value="flash">点滅</option>
                    <option value="pulse">パルス</option>
                  </select>
                </label>
                <Note>単独レイヤー時に本体へ委譲します。色と輝度のプレビューは近似値です。</Note>
              </>
            )}
            <div className="divider" />
            <h4>合成</h4>
            <label className="field">
              <span>ブレンドモード</span>
              <select
                value={layer.blend}
                onChange={(e) => patch({ blend: e.target.value as Blend })}
              >
                {Object.entries(blends).map(([k, v]) => (
                  <option key={k} value={k}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
            <Slider
              label="不透明度"
              value={layer.opacity}
              onChange={(v) => patch({ opacity: v })}
            />
            <div className="divider" />
            <Slider
              label="プリセット輝度"
              value={state.preset.post.brightness}
              onChange={(v) =>
                edit({ ...state.preset, post: { ...state.preset.post, brightness: v } })
              }
            />
            <div className="inspector-bottom">
              <div className="live-hint">
                <span className="tiny-dot" />
                変更はすぐにプレビューへ反映
              </div>
              <button className="primary full" onClick={onSave}>
                <Save size={16} />
                プリセットを保存
              </button>
            </div>
          </>
        ) : (
          <Note>レイヤーを追加すると、色や速度を編集できます。</Note>
        )}
      </aside>
      {adding && (
        <Modal title="エフェクトを追加" onClose={() => setAdding(false)} wide>
          <div className="effect-grid">
            {Object.entries(effects).map(([id, name]) => (
              <button
                key={id}
                onClick={() => {
                  const l: Layer = {
                    id: uid(),
                    effect: id,
                    params: { color: '#43ffc2', speed: 0.6 },
                    zone: 'all',
                    opacity: 1,
                    blend: ['ripple', 'reactive', 'starlight'].includes(id) ? 'add' : 'normal',
                    enabled: true,
                  };
                  change({ ...state.preset, layers: [l, ...state.preset.layers] });
                  setSelectedLayer(l.id);
                  setAdding(false);
                }}
              >
                <Layers3 size={20} />
                <strong>{name}</strong>
                <small>{id}</small>
              </button>
            ))}
          </div>
        </Modal>
      )}
    </div>
  );
}
