import { useState, useRef, useEffect } from 'react';
import {
  Search,
  Plus,
  Download,
  Upload,
  Copy,
  Trash2,
  Check,
  ArrowUpRight,
  Radio,
  ShieldCheck,
  Layers3,
  Usb,
  RefreshCw,
  Play,
  Moon,
  Clock3,
  SlidersHorizontal,
  ChevronRight,
  ChevronLeft,
  Save,
  Monitor,
} from 'lucide-react';
import type { ViewProps } from './ui-state';
import type { Preset, Profile, Layout, DisplaySettings } from './types';
import { uid } from './types';
import { native, exportJson, openLink } from './api';
import { renderPreview, ditherImage } from './preview';
import { Card, PageTitle, Note, Toggle, Slider, Modal } from './components';
import { appVersion } from './version';
import HardwareFace from './HardwareFace';
import { UpdatesCard, type UpdateController } from './Updates';
const serviceUrl = 'https://microsoft.github.io/MIDI/get-latest/';
const loopbackUrl = 'https://microsoft.github.io/MIDI/kb/virtual-loopback/';
export function Miniature({ preset, layout }: { preset: Preset; layout: Layout }) {
  return (
    <svg viewBox={`0 0 ${layout.canvas.w} ${layout.canvas.h}`} aria-hidden="true">
      <HardwareFace
        layout={layout}
        colors={renderPreview(preset, layout, 18.4, 1)}
        gamma={preset.post.gamma}
      />
    </svg>
  );
}
export function PresetsScreen({ state, act, toast }: ViewProps) {
  const [search, setSearch] = useState(''),
    [filter, setFilter] = useState('all');
  const file = useRef<HTMLInputElement>(null);
  const exportPreset = async (p: Preset) => {
    try {
      await exportJson(`${p.id}.keylume.json`, p);
      toast('プリセットを書き出しました');
    } catch (e) {
      toast(String(e));
    }
  };
  return (
    <div className="page">
      <PageTitle
        eyebrow="ライブラリ"
        title="プリセット"
        description="プリセットの適用、複製、読み込みと書き出し。"
      >
        <button className="secondary" onClick={() => file.current?.click()}>
          <Upload size={16} />
          インポート
        </button>
        <input
          ref={file}
          type="file"
          accept=".json"
          hidden
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (f) {
              if (f.size > 2_000_000) {
                toast('ファイルは 2 MB 以下にしてください');
                return;
              }
              if (await act('import_preset', { json: await f.text() }))
                toast('プリセットを読み込みました');
            }
            e.target.value = '';
          }}
        />
      </PageTitle>
      <div className="filter-row">
        <div className="segmented">
          {[
            ['all', 'すべて'],
            ['builtin', '標準プリセット'],
            ['custom', 'マイプリセット'],
          ].map(([id, label]) => (
            <button
              key={id}
              className={filter === id ? 'selected' : ''}
              onClick={() => setFilter(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <label className="search">
          <Search size={16} />
          <input
            aria-label="プリセットを検索"
            placeholder="プリセットを検索"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
      </div>
      <div className="preset-grid">
        {state.presets
          .filter(
            (p) =>
              (filter === 'all' || (filter === 'builtin' ? p.builtin : !p.builtin)) &&
              p.name.toLowerCase().includes(search.toLowerCase()),
          )
          .map((p) => (
            <article
              key={p.id}
              className={'preset-card ' + (state.preset.id === p.id ? 'selected' : '')}
            >
              <button
                className="preset-apply"
                onClick={() => void act('apply_preset', { id: p.id })}
              >
                <div className={'preset-art art-' + p.id}>
                  <Miniature preset={p} layout={state.layout} />
                  <span className="preset-play">
                    {state.preset.id === p.id ? <Check size={18} /> : <Play size={18} />}
                  </span>
                </div>
                <div className="preset-name">
                  <h3>{p.name}</h3>
                  <small>
                    {p.layers.length} レイヤー · {p.builtin ? '標準' : 'カスタム'}
                  </small>
                </div>
              </button>
              <div className="preset-actions">
                <span>{state.preset.id === p.id ? '適用中' : ' '}</span>
                <button
                  className="icon-button"
                  aria-label={`${p.name}を複製`}
                  onClick={async () => {
                    if (
                      await act('save_preset', {
                        preset: { ...p, id: uid(), name: p.name + ' のコピー', builtin: false },
                      })
                    )
                      toast('プリセットを複製しました');
                  }}
                >
                  <Copy size={15} />
                </button>
                <button
                  className="icon-button"
                  aria-label={`${p.name}を書き出す`}
                  onClick={() => void exportPreset(p)}
                >
                  <Download size={15} />
                </button>
                {!p.builtin && (
                  <button
                    className="icon-button"
                    aria-label={`${p.name}を削除`}
                    onClick={() => void act('delete_preset', { id: p.id })}
                  >
                    <Trash2 size={15} />
                  </button>
                )}
              </div>
            </article>
          ))}
      </div>
      {state.presets.filter(
        (p) =>
          (filter === 'all' || (filter === 'builtin' ? p.builtin : !p.builtin)) &&
          p.name.toLowerCase().includes(search.toLowerCase()),
      ).length === 0 && (
        <div className="empty-state">
          <Search />
          <h3>プリセットが見つかりません</h3>
          <p>検索条件を変えるか、プリセットを保存してください。</p>
        </div>
      )}
    </div>
  );
}
export function ProfilesScreen({ state, act, saveSettings, toast }: ViewProps) {
  const [draft, setDraft] = useState<Profile | null>(null);
  return (
    <div className="page">
      <PageTitle
        eyebrow="オートメーション"
        title="プロファイル"
        description="アプリ、時間帯、アイドル状態に応じた自動切り替え。"
      >
        <button
          className="primary"
          onClick={() =>
            setDraft({
              id: uid(),
              name: '新しいプロファイル',
              match: { processes: ['reaper.exe'] },
              presetId: state.presets[0].id,
              coexistMode: 'handoff',
              priority: 0,
            })
          }
        >
          <Plus size={16} />
          プロファイルを追加
        </button>
      </PageTitle>
      <Card>
        <Toggle
          label="手動で固定する"
          description="ON の間は自動切替を停止し、選択したプリセットを使います。"
          checked={state.settings.manualLock}
          onChange={(v) => saveSettings({ ...state.settings, manualLock: v })}
        />
        <div className="active-profile">
          <span className="tiny-dot" />
          {state.settings.manualLock
            ? '手動固定中'
            : (state.status.activeProfile ?? '既定のプリセットを使用中')}
        </div>
      </Card>
      <div className="priority-line">
        手動固定 <ChevronRight size={13} /> 前面アプリ <ChevronRight size={13} /> 起動中のアプリ{' '}
        <ChevronRight size={13} /> 時間 / アイドル <ChevronRight size={13} /> 既定
      </div>
      {!state.profiles.length && (
        <div className="empty-state large">
          <Clock3 size={32} />
          <h2>あなたの制作時間に合わせて。</h2>
          <p>
            DAW を開いたらライティングを譲る。夜になったら暖かな色に。
            <br />
            最初のプロファイルを追加してみましょう。
          </p>
          <button
            className="secondary"
            onClick={() =>
              setDraft({
                id: uid(),
                name: '夜の制作',
                match: { timeRange: ['23:00', '07:00'] },
                presetId: 'night',
                coexistMode: 'lightingFirst',
                priority: 0,
              })
            }
          >
            <Moon size={16} />
            夜間プロファイルを作る
          </button>
        </div>
      )}
      {state.profiles.map((p) => (
        <Card key={p.id}>
          <div className="profile-row">
            <div className="effect-icon">
              <SlidersHorizontal size={21} />
            </div>
            <div className="grow">
              <h3>{p.name}</h3>
              <p className="muted">
                {p.match.processes?.join(', ') ??
                  p.match.timeRange?.join(' → ') ??
                  `アイドル ${p.match.idleMinutes ?? 0} 分`}{' '}
                · {state.presets.find((x) => x.id === p.presetId)?.name} · 優先度 {p.priority}
              </p>
            </div>
            <button className="secondary" onClick={() => setDraft(structuredClone(p))}>
              編集
            </button>
            <button
              className="icon-button"
              aria-label={`${p.name}を削除`}
              onClick={() => void act('delete_profile', { id: p.id })}
            >
              <Trash2 size={17} />
            </button>
          </div>
        </Card>
      ))}
      {draft && (
        <Modal title="プロファイルを編集" onClose={() => setDraft(null)}>
          <label className="field">
            <span>名前</span>
            <input
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </label>
          <label className="field">
            <span>切替条件</span>
            <select
              value={draft.match.processes ? 'process' : draft.match.timeRange ? 'time' : 'idle'}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  match:
                    e.target.value === 'process'
                      ? { processes: ['reaper.exe'] }
                      : e.target.value === 'time'
                        ? { timeRange: ['23:00', '07:00'] }
                        : { idleMinutes: 10 },
                })
              }
            >
              <option value="process">アプリの起動 / 前面</option>
              <option value="time">時間帯</option>
              <option value="idle">PC のアイドル</option>
            </select>
          </label>
          {draft.match.processes && (
            <>
              <label className="field">
                <span>実行ファイル名（1 行に 1 つ、* 使用可）</span>
                <textarea
                  value={draft.match.processes.join('\n')}
                  onChange={(e) =>
                    setDraft({
                      ...draft,
                      match: { ...draft.match, processes: e.target.value.split('\n') },
                    })
                  }
                />
              </label>
              <Toggle
                label="前面のアプリだけに適用"
                checked={draft.match.foregroundOnly ?? false}
                onChange={(v) =>
                  setDraft({ ...draft, match: { ...draft.match, foregroundOnly: v } })
                }
              />
            </>
          )}
          {draft.match.timeRange && (
            <div className="two-columns">
              {draft.match.timeRange.map((v, i) => (
                <label key={i} className="field">
                  <span>{i ? '終了' : '開始'}</span>
                  <input
                    type="time"
                    value={v}
                    onChange={(e) => {
                      const r = [...draft.match.timeRange!] as [string, string];
                      r[i] = e.target.value;
                      setDraft({ ...draft, match: { timeRange: r } });
                    }}
                  />
                </label>
              ))}
            </div>
          )}
          {draft.match.idleMinutes !== undefined && (
            <label className="field">
              <span>アイドル時間（分）</span>
              <input
                type="number"
                min="1"
                max="1440"
                value={draft.match.idleMinutes}
                onChange={(e) =>
                  setDraft({ ...draft, match: { idleMinutes: Number(e.target.value) } })
                }
              />
            </label>
          )}
          <label className="field">
            <span>プリセット</span>
            <select
              value={draft.presetId}
              onChange={(e) => setDraft({ ...draft, presetId: e.target.value })}
            >
              {state.presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>共存モード</span>
            <select
              value={draft.coexistMode}
              onChange={(e) =>
                setDraft({ ...draft, coexistMode: e.target.value as Profile['coexistMode'] })
              }
            >
              <option value="lightingFirst">ライティング優先</option>
              <option value="handoff">DAW に譲る</option>
            </select>
          </label>
          <label className="field">
            <span>同じ種類の条件内での優先度</span>
            <input
              type="number"
              value={draft.priority}
              onChange={(e) => setDraft({ ...draft, priority: Number(e.target.value) })}
            />
          </label>
          <button
            className="primary full"
            onClick={async () => {
              if (await act('save_profile', { profile: draft })) {
                setDraft(null);
                toast('プロファイルを保存しました');
              }
            }}
          >
            <Save size={16} />
            プロファイルを保存
          </button>
        </Modal>
      )}
    </div>
  );
}
const dawGuides: Record<string, string> = {
  Ableton:
    '環境設定 → Link, Tempo & MIDI で Launchkey のコントロールサーフェスを「None」にします。MIDI 入力の Track を鍵盤ポートと各ループバックの B 側入力で ON にします。',
  FLStudio:
    'Options → MIDI settings で Launchkey のネイティブスクリプトを解除します。鍵盤とループバック B 側入力を有効にし、Controller type を汎用コントローラーに設定します。',
  REAPER:
    'Preferences → MIDI Devices で鍵盤とループバック B 側入力を有効にします。Control/OSC/web で Launchkey のネイティブ制御を解除します。',
  Other:
    'DAW の MIDI / Control Surface 設定で Launchkey のネイティブ連携を解除し、鍵盤 MIDI 入力とループバック B 側入力を有効にします。設定名はバージョンにより異なります。',
};
export function CoexistScreen({ state, act, saveSettings }: ViewProps) {
  const [daw, setDaw] = useState('Ableton');
  const s = state.settings;
  return (
    <div className="page">
      <PageTitle
        eyebrow="つながりを整える"
        title="DAW との共存"
        description="演奏の流れをそのままに。ライティングの役割を選びます。"
      />
      <div className="mode-grid">
        {[
          {
            id: 'lightingFirst',
            icon: Radio,
            title: 'ライティング優先',
            text: 'Keylume が光を管理。パッドやコントロールの入力を DAW へ転送します。',
            tag: 'おすすめ',
          },
          {
            id: 'handoff',
            icon: ShieldCheck,
            title: 'DAW に譲る',
            text: 'DAW の起動中はポートを解放。ネイティブ連携を優先します。',
            tag: '制御サーフェスを使う',
          },
        ].map((m) => (
          <button
            key={m.id}
            className={'mode-card ' + (s.coexistMode === m.id ? 'selected' : '')}
            onClick={() => void act('set_coexist_mode', { mode: m.id })}
          >
            <m.icon size={25} />
            <span className="tag">{m.tag}</span>
            <h3>{m.title}</h3>
            <p>{m.text}</p>
            <span className="radio-dot">{s.coexistMode === m.id && <Check size={12} />}</span>
          </button>
        ))}
        <div className="mode-card disabled">
          <Layers3 size={25} />
          <span className="tag">Phase 3</span>
          <h3>レイヤー合成</h3>
          <p>DAW の表示とライティングを合成。将来の拡張機能です。</p>
        </div>
      </div>
      <div className="two-columns">
        <Card title="MIDI 環境">
          <div className="status-line">
            <span>Windows MIDI Services</span>
            <span className={'badge ' + (state.status.service === 'running' ? 'ok' : 'warn')}>
              {state.status.service === 'running'
                ? 'サービス稼働'
                : state.status.service === 'browser'
                  ? 'デスクトップ版で確認'
                  : '未検出 / 停止'}
            </span>
          </div>
          <p className="muted">
            サービスの稼働だけでは、WinMM の同時利用は保証されません。DAW
            との実動作を確認してください。
          </p>
          <button className="link-button" onClick={() => void openLink(serviceUrl)}>
            Microsoft のセットアップ案内
            <ArrowUpRight size={14} />
          </button>
        </Card>
        <Card title="DAW の検出">
          <div className="detected-list">
            {state.status.dawActive.length ? (
              state.status.dawActive.map((p) => (
                <span key={p} className="badge ok">
                  {p}
                </span>
              ))
            ) : (
              <span className="muted">対象の DAW は起動していません</span>
            )}
          </div>
          <label className="field">
            <span>検出する実行ファイル（1 行に 1 つ）</span>
            <textarea
              rows={3}
              value={s.dawProcesses.join('\n')}
              onChange={(e) => saveSettings({ ...s, dawProcesses: e.target.value.split('\n') })}
            />
          </label>
        </Card>
      </div>
      {s.coexistMode === 'lightingFirst' ? (
        <>
          <Card
            title="演奏データの転送"
            description="MIDI Services で作成したループバックの A 側出力を選択してください。DAW は対応する B 側入力を使います。"
          >
            <Toggle
              label="入力転送を有効にする"
              checked={s.forwarding}
              onChange={(v) => saveSettings({ ...s, forwarding: v })}
            />
            <div className="two-columns">
              {[
                ['padsPort', 'Keylume Pads', state.status.padsPort],
                ['controlsPort', 'Keylume Controls', state.status.controlsPort],
              ].map(([key, label, found]) => (
                <label className="field" key={String(key)}>
                  <span>
                    {label}
                    <small className={found ? 'success' : 'muted'}>
                      {found ? '接続済み' : '未接続'}
                    </small>
                  </span>
                  <select
                    value={s[key as 'padsPort' | 'controlsPort']}
                    onChange={(e) => saveSettings({ ...s, [String(key)]: e.target.value })}
                  >
                    <option value={s[key as 'padsPort' | 'controlsPort']}>
                      {s[key as 'padsPort' | 'controlsPort']}（選択中）
                    </option>
                    {state.status.ports.outputs
                      .filter(
                        (p) =>
                          !p.toLowerCase().includes('launchkey') &&
                          p !== s[key as 'padsPort' | 'controlsPort'],
                      )
                      .map((p) => (
                        <option key={p}>{p}</option>
                      ))}
                  </select>
                </label>
              ))}
            </div>
            <button className="link-button" onClick={() => void openLink(loopbackUrl)}>
              ループバックの作成手順
              <ArrowUpRight size={14} />
            </button>
            <div className="routing-flow">
              <span>Launchkey DAW 入力</span>
              <ChevronRight size={16} />
              <span>Keylume · A 出力</span>
              <ChevronRight size={16} />
              <span>DAW · B 入力</span>
            </div>
          </Card>
          <Card title="入力のリマップ">
            <div className="two-columns">
              <label className="field">
                <span>パッドの送信チャンネル</span>
                <select
                  value={s.padChannel}
                  onChange={(e) => saveSettings({ ...s, padChannel: Number(e.target.value) })}
                >
                  {Array.from({ length: 16 }, (_, i) => (
                    <option key={i} value={i}>
                      Ch {i + 1}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>コントロールの送信チャンネル</span>
                <select
                  value={s.controlChannel ?? -1}
                  onChange={(e) =>
                    saveSettings({
                      ...s,
                      controlChannel: Number(e.target.value) < 0 ? null : Number(e.target.value),
                    })
                  }
                >
                  <option value={-1}>そのまま</option>
                  {Array.from({ length: 16 }, (_, i) => (
                    <option key={i} value={i}>
                      Ch {i + 1}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="pad-map">
              {s.padNotes.map((n, i) => (
                <label key={i}>
                  <span>
                    {i < 8 ? '上' : '下'} {(i % 8) + 1}
                  </span>
                  <input
                    aria-label={`パッド${i < 8 ? '上' : '下'}段${(i % 8) + 1}のノート`}
                    type="number"
                    min={0}
                    max={127}
                    value={n}
                    onChange={(e) => {
                      const notes = [...s.padNotes];
                      notes[i] = Number(e.target.value);
                      saveSettings({ ...s, padNotes: notes });
                    }}
                  />
                </label>
              ))}
            </div>
            <Toggle
              label="パッドのアフタータッチを転送"
              checked={s.aftertouch}
              onChange={(v) => saveSettings({ ...s, aftertouch: v })}
            />
            <details>
              <summary>コントロールの詳細リマップ</summary>
              <label className="field">
                <span>CC 番号の変換（JSON、例: {'{"85": 1}'}）</span>
                <textarea
                  defaultValue={JSON.stringify(s.ccMap)}
                  onBlur={(e) => {
                    try {
                      saveSettings({ ...s, ccMap: JSON.parse(e.target.value) });
                    } catch {
                      e.target.setCustomValidity('有効な JSON を入力してください');
                      e.target.reportValidity();
                    }
                  }}
                />
              </label>
              <label className="field">
                <span>転送しない CC 番号（カンマ区切り）</span>
                <input
                  defaultValue={s.shortcutCcs.join(',')}
                  onBlur={(e) =>
                    saveSettings({
                      ...s,
                      shortcutCcs: e.target.value.split(',').filter(Boolean).map(Number),
                    })
                  }
                />
              </label>
            </details>
          </Card>
        </>
      ) : (
        <Note>
          DAW 起動時に消灯・DAW モード解除・ポート解放を行います。対象 DAW の終了から 3
          秒後にライティングを再開します。
        </Note>
      )}
      <Card title="DAW 側のセットアップ">
        <select aria-label="使用する DAW" value={daw} onChange={(e) => setDaw(e.target.value)}>
          <option value="Ableton">Ableton Live</option>
          <option value="FLStudio">FL Studio</option>
          <option value="REAPER">REAPER</option>
          <option value="Other">その他の DAW</option>
        </select>
        <p className="guide-text">
          {s.coexistMode === 'handoff'
            ? 'DAW の通常の Launchkey 制御サーフェス設定を利用できます。入力転送用ポートは不要です。'
            : dawGuides[daw]}
        </p>
        <p className="micro">
          Components のカスタムモードでは、フェーダーやノブのデータを鍵盤 MIDI
          ポートへ直接出せます。
        </p>
      </Card>
    </div>
  );
}
function Bitmap({ bits }: { bits: number[] }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const ctx = ref.current?.getContext('2d');
    if (!ctx) return;
    const image = ctx.createImageData(128, 64);
    bits.forEach((b, i) => {
      image.data.set(b ? [173, 230, 211, 255] : [8, 19, 18, 255], i * 4);
    });
    ctx.putImageData(image, 0, 0);
  }, [bits]);
  return <canvas className="oled-bitmap" ref={ref} width={128} height={64} />;
}
export function DeviceScreen({ state, act, edit, saveSettings, toast }: ViewProps) {
  const [index, setIndex] = useState(0),
    [variant, setVariant] = useState(0),
    [layoutJson, setLayoutJson] = useState<string | null>(null),
    [level, setLevel] = useState(100),
    [screenLevel, setScreenLevel] = useState(80);
  const selectedIndex = Math.min(index, state.layout.leds.length - 1);
  const led = state.layout.leds[selectedIndex];
  const display: DisplaySettings = state.preset.display ?? {
    enabled: false,
    widget: 'presetName',
    showOnPresetChange: true,
  };
  const imageInput = useRef<HTMLInputElement>(null);
  const updateDisplay = (next: Partial<DisplaySettings>) =>
    edit({ ...state.preset, display: { ...display, ...next } });
  const probe = () => void act('run_led_probe', { id: led.id, variant });
  const address = (field: string, value: number) => {
    const layout = structuredClone(state.layout);
    layout.leds[selectedIndex].address = { ...led.address, [field]: value };
    layout.leds[selectedIndex].verified = false;
    void act('save_layout', { layout });
  };
  return (
    <div className="page">
      <PageTitle
        eyebrow="ハードウェア"
        title="デバイス"
        description="Launchkey MK4 61 の接続、OLED、LED アドレスの設定。"
      >
        <button className="secondary" onClick={() => void act('reconnect')}>
          <RefreshCw size={16} />
          再接続
        </button>
      </PageTitle>
      <Card>
        <div className="device-summary">
          <span className="device-emblem">
            <Usb size={30} />
          </span>
          <div className="grow">
            <h2>Launchkey MK4 61</h2>
            <p className="muted">{state.status.deviceName}</p>
          </div>
          <span className="badge">
            {state.settings.mock
              ? 'プレビューデバイス'
              : state.status.connection === 'connected'
                ? '接続中'
                : '未接続'}
          </span>
        </div>
        <Toggle
          label="MockDevice でプレビュー"
          checked={state.settings.mock}
          description="OFF にすると実機の DAW ポートを検出します。"
          onChange={(v) => saveSettings({ ...state.settings, mock: v })}
        />
        <div className="diagnostic-grid">
          <div>
            <span>Inquiry 応答</span>
            <code>{state.status.inquiry || '応答待ち'}</code>
          </div>
          <div>
            <span>パッドモード</span>
            <code>0x{state.status.padMode.toString(16).padStart(2, '0').toUpperCase()}</code>
          </div>
          <div>
            <span>送信メッセージ</span>
            <strong className="numeric">{state.status.messages.toLocaleString()}</strong>
          </div>
          <div>
            <span>送信バイト</span>
            <strong className="numeric">{state.status.bytes.toLocaleString()}</strong>
          </div>
        </div>
      </Card>
      {state.status.warnings.length > 0 && (
        <Card title="診断メッセージ">
          <ul className="warning-list">
            {state.status.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </Card>
      )}
      <div className="two-columns">
        <Card
          title="本体の設定"
          description="本体に保存される値です。操作後 1 秒で送信します。マスター輝度とは独立しています。"
        >
          <Slider
            label="本体の最大輝度"
            value={level}
            min={0}
            max={127}
            step={1}
            display={String(level)}
            onChange={(v) => {
              setLevel(v);
              void act('set_device_feature', { cc: 111, value: v });
            }}
          />
          <Slider
            label="ディスプレイの輝度"
            value={screenLevel}
            min={0}
            max={127}
            step={1}
            display={String(screenLevel)}
            onChange={(v) => {
              setScreenLevel(v);
              void act('set_device_feature', { cc: 112, value: v });
            }}
          />
          <Toggle
            label="DAW Drum でも光らせる"
            checked={state.settings.dawDrum}
            onChange={(v) => saveSettings({ ...state.settings, dawDrum: v })}
          />
          <p className="micro">RGB アドレスは実機での確認が必要です。</p>
          <div className="button-row">
            <button
              className="subtle"
              onClick={() => void act('set_device_feature', { cc: 114, value: 127 })}
            >
              本体デモを ON（未検証）
            </button>
            <button
              className="subtle"
              onClick={() => void act('set_device_feature', { cc: 114, value: 0 })}
            >
              OFF
            </button>
          </div>
        </Card>
        <Card title="OLED ディスプレイ">
          <Toggle
            label="OLED を使う"
            checked={display.enabled}
            onChange={(v) => updateDisplay({ enabled: v })}
          />
          <label className="field">
            <span>表示するウィジェット</span>
            <select
              value={display.widget}
              onChange={(e) =>
                updateDisplay({ widget: e.target.value as DisplaySettings['widget'] })
              }
            >
              <option value="presetName">プリセット ID（ASCII）</option>
              <option value="clock">時計</option>
              <option value="miniSpectrum">ミニスペクトラム（実験的）</option>
              <option value="image">カスタム画像（実験的）</option>
              <option value="off">表示しない</option>
            </select>
          </label>
          {display.widget === 'image' && (
            <>
              <button className="secondary" onClick={() => imageInput.current?.click()}>
                <Upload size={15} />
                画像を読み込む
              </button>
              <input
                type="file"
                hidden
                ref={imageInput}
                accept="image/png,image/jpeg"
                onChange={async (e) => {
                  const f = e.target.files?.[0];
                  if (!f) return;
                  if (f.size > 20_000_000) {
                    toast('画像は 20 MB 以下にしてください');
                    return;
                  }
                  try {
                    const bitmap = await createImageBitmap(f);
                    const canvas = document.createElement('canvas');
                    canvas.width = 128;
                    canvas.height = 64;
                    const ctx = canvas.getContext('2d')!;
                    ctx.fillStyle = '#000';
                    ctx.fillRect(0, 0, 128, 64);
                    ctx.drawImage(bitmap, 0, 0, 128, 64);
                    bitmap.close();
                    updateDisplay({ imageBits: ditherImage(ctx.getImageData(0, 0, 128, 64).data) });
                  } catch (e) {
                    toast(`画像を読み込めません: ${e}`);
                  }
                }}
              />
              {display.imageBits && <Bitmap bits={display.imageBits} />}
            </>
          )}
          <p className="micro">
            画像は 128 × 64 の 1bit
            に変換します。実機の応答がない場合は画像送信を停止します。変更はプリセットに保存してください。
          </p>
        </Card>
      </div>
      <Card
        title="LED マップ検証"
        description="候補を点灯させ、実機を見て回答します。プレビューでの回答は「実機検証済み」にはなりません。"
      >
        <div className="probe-header">
          <label className="field grow">
            <span>
              検証する LED · {selectedIndex + 1} / {state.layout.leds.length}
            </span>
            <select
              value={selectedIndex}
              onChange={(e) => {
                void act('stop_led_probe');
                setIndex(Number(e.target.value));
              }}
            >
              {state.layout.leds.map((l, i) => (
                <option value={i} key={l.id}>
                  {l.verified ? '✓ ' : ''}
                  {l.id} · {l.kind}
                </option>
              ))}
            </select>
          </label>
          <button
            className="icon-button"
            aria-label="前の LED"
            disabled={selectedIndex === 0}
            onClick={() => {
              void act('stop_led_probe');
              setIndex(selectedIndex - 1);
            }}
          >
            <ChevronLeft size={18} />
          </button>
          <button
            className="icon-button"
            aria-label="次の LED"
            disabled={selectedIndex === state.layout.leds.length - 1}
            onClick={() => {
              void act('stop_led_probe');
              setIndex(selectedIndex + 1);
            }}
          >
            <ChevronRight size={18} />
          </button>
        </div>
        <div className="two-columns">
          <label className="field">
            <span>テストするメッセージ</span>
            <select value={variant} onChange={(e) => setVariant(Number(e.target.value))}>
              <option value={0}>RGB（赤 → 緑 → 青）</option>
              <option value={1}>単色 · B3</option>
              <option value={2}>単色 · 93</option>
            </select>
          </label>
          <div className="address-row">
            {Object.entries(led.address)
              .filter(([k]) => k !== 'monoStatus' && k !== 'drumNote')
              .map(([k, v]) => (
                <label key={k} className="field">
                  <span>{k}</span>
                  <input
                    aria-label={`${led.id} ${k}`}
                    type="number"
                    min={0}
                    max={127}
                    value={v}
                    onChange={(e) => address(k, Number(e.target.value))}
                  />
                </label>
              ))}
          </div>
        </div>
        <div className="button-row">
          <button className="primary" onClick={probe}>
            <Play size={15} />
            {state.status.probe === led.id ? 'テストを再実行' : '点灯テスト'}
          </button>
          <button className="secondary" onClick={() => void act('stop_led_probe')}>
            テストを停止
          </button>
          <span className="micro">{led.verified ? '実機検証済み' : '実機未検証'}</span>
        </div>
        <div className="probe-answers">
          {[
            ['rgb', '光った（色付き）'],
            ['mono', '光った（白のみ）'],
            ['none', '光らない'],
          ].map(([kind, label]) => (
            <button
              className="secondary"
              key={kind}
              onClick={async () => {
                if (
                  await act('answer_led_probe', {
                    id: led.id,
                    kind,
                    monoStatus: variant === 2 ? 147 : 179,
                  })
                ) {
                  toast('検証結果を保存しました');
                  setIndex(Math.min(state.layout.leds.length - 1, selectedIndex + 1));
                }
              }}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="divider" />
        <div className="button-row">
          <button
            className="subtle"
            onClick={() => void exportJson('launchkey-mk4-61.layout.json', state.layout)}
          >
            <Download size={15} />
            レイアウトを書き出す
          </button>
          <button
            className="subtle"
            onClick={() => setLayoutJson(JSON.stringify(state.layout, null, 2))}
          >
            レイアウト JSON を編集
          </button>
        </div>
      </Card>
      <Card title="MIDI モニタ">
        <Toggle
          label="診断ログを有効にする"
          description="最大 100 メッセージを表示します。"
          checked={state.settings.midiLog}
          onChange={(v) => saveSettings({ ...state.settings, midiLog: v })}
        />
        <pre className="midi-monitor">
          {state.status.monitor.length
            ? state.status.monitor.join('\n')
            : 'メッセージを待っています…'}
        </pre>
      </Card>
      {state.debug && state.settings.mock && (
        <Card title="開発用シミュレーション">
          <div className="button-row">
            <button
              className="secondary"
              onClick={() =>
                void act('mock_daw', { value: !state.status.dawActive.includes('Mock DAW') })
              }
            >
              DAW 起動 / 終了
            </button>
            <button
              className="secondary"
              onClick={() =>
                void act('mock_disconnect', { value: state.status.connection !== 'disconnected' })
              }
            >
              切断 / 再接続
            </button>
            <button
              className="secondary"
              onClick={() =>
                void act('simulate_input', {
                  source: 'daw',
                  bytes: [182, 29, state.status.padMode === 2 ? 5 : 2],
                })
              }
            >
              Custom / DAW モード
            </button>
          </div>
        </Card>
      )}
      {layoutJson !== null && (
        <Modal wide title="デバイスレイアウト" onClose={() => setLayoutJson(null)}>
          <Note>
            アドレスは 10 進数です。エンコーダーページ下は 52 / 68（0x34 / 0x44）を比較できます。
          </Note>
          <textarea
            className="json-editor"
            aria-label="レイアウト JSON"
            value={layoutJson}
            onChange={(e) => setLayoutJson(e.target.value)}
          />
          <button
            className="primary full"
            onClick={async () => {
              try {
                if (await act('save_layout', { layout: JSON.parse(layoutJson) }))
                  setLayoutJson(null);
              } catch (e) {
                toast(String(e));
              }
            }}
          >
            レイアウトを保存
          </button>
        </Modal>
      )}
    </div>
  );
}
export function SettingsScreen({
  state,
  saveSettings,
  edit,
  toast,
  onSetup,
  updates,
}: ViewProps & { onSetup: () => void; updates: UpdateController }) {
  const [autostart, setAutostart] = useState(false);
  useEffect(() => {
    if (native)
      void import('@tauri-apps/plugin-autostart')
        .then((m) => m.isEnabled())
        .then(setAutostart)
        .catch((e) => toast(String(e)));
  }, [toast]);
  const s = state.settings;
  return (
    <div className="page">
      <PageTitle
        eyebrow="アプリケーション"
        title="設定"
        description="起動、描画、音声入出力、更新通知。"
      />
      <div className="two-columns">
        <UpdatesCard updates={updates} settings={s} saveSettings={saveSettings} />
        <Card title="起動と常駐">
          <Toggle
            label="Windows 起動時に自動起動"
            description="トレイに最小化した状態で開始します。"
            checked={autostart}
            disabled={!native}
            onChange={async (v) => {
              try {
                const a = await import('@tauri-apps/plugin-autostart');
                await (v ? a.enable() : a.disable());
                setAutostart(v);
              } catch (e) {
                toast(String(e));
              }
            }}
          />
          <Note>
            ウィンドウを閉じてもライティングは続きます。終了するときはトレイメニューから「終了」を選んでください。
          </Note>
          <button className="secondary" onClick={onSetup}>
            セットアップを開く
          </button>
        </Card>
        <Card title="描画と接続">
          <label className="field">
            <span>実機への送信レート</span>
            <select
              value={s.fps}
              onChange={(e) => saveSettings({ ...s, fps: Number(e.target.value) })}
            >
              {[15, 30, 60].map((n) => (
                <option key={n} value={n}>
                  {n} fps{n === 30 ? '（推奨）' : ''}
                </option>
              ))}
            </select>
          </label>
          <Toggle
            label="ライティングを自動修復"
            description="モード A で 3 秒ごとにフルフレームを再送します。"
            checked={s.autoRepair}
            onChange={(v) => saveSettings({ ...s, autoRepair: v })}
          />
          <Toggle
            label="停止時にフェードアウト"
            checked={s.fadeOnRelease}
            onChange={(v) => saveSettings({ ...s, fadeOnRelease: v })}
          />
          <Toggle
            label="鍵盤入力に反応する"
            checked={s.keyboardReactive}
            onChange={(v) => saveSettings({ ...s, keyboardReactive: v })}
          />
        </Card>
        <Card title="プリセットの色補正">
          <Slider
            label="ガンマ"
            min={0.1}
            max={4}
            step={0.1}
            value={state.preset.post.gamma}
            display={state.preset.post.gamma.toFixed(1)}
            onChange={(v) => edit({ ...state.preset, post: { ...state.preset.post, gamma: v } })}
          />
          <Slider
            label="彩度"
            min={0}
            max={2}
            value={state.preset.post.saturation}
            onChange={(v) =>
              edit({ ...state.preset, post: { ...state.preset.post, saturation: v } })
            }
          />
          <Slider
            label="色温度"
            min={1000}
            max={12000}
            step={100}
            value={state.preset.post.temperatureK}
            display={`${state.preset.post.temperatureK} K`}
            onChange={(v) =>
              edit({ ...state.preset, post: { ...state.preset.post, temperatureK: v } })
            }
          />
          <p className="micro">色補正は編集中のプリセットに適用されます。</p>
        </Card>
        <Card title="楽器の音声出力">
          <label className="field">
            <span>ピアノ・ドラムの出力先</span>
            <select
              aria-label="楽器の音声出力先"
              value={s.piano.outputDevice}
              onChange={(e) =>
                saveSettings({ ...s, piano: { ...s.piano, outputDevice: e.target.value } })
              }
            >
              <option value="">Windows の既定に自動追従</option>
              {[
                ...new Set([
                  ...state.status.audioDevices,
                  ...(s.piano.outputDevice ? [s.piano.outputDevice] : []),
                ]),
              ].map((d) => (
                <option key={d}>{d}</option>
              ))}
            </select>
          </label>
          <p className="muted">
            「Windows
            の既定」は、ヘッドホンやスピーカーへの出力変更に約1秒で追従します。デバイスを指定すると、その出力先を使い続けます。
          </p>
          <Note>
            出力切り替え中は一時的に音が途切れます。ループの録音内容は保持します。ピアノOFFまたはアプリ終了で消去されます。
          </Note>
        </Card>
        <Card title="音声入力">
          <label className="field">
            <span>ループバックする再生デバイス</span>
            <select
              value={s.audioDevice}
              onChange={(e) => saveSettings({ ...s, audioDevice: e.target.value })}
            >
              <option value="">Windows の既定の出力</option>
              {state.status.audioDevices.map((d) => (
                <option key={d}>{d}</option>
              ))}
            </select>
          </label>
          <p className="muted">
            音声系エフェクトを使う間だけ WASAPI ループバックを開始します。FFT 2048、8
            バンド、自動ゲインと平滑化を使用します。
          </p>
          <span className="badge">
            {state.status.audio === 'capturing'
              ? '解析中'
              : state.status.audio === 'stopped'
                ? '停止中'
                : state.status.audio}
          </span>
        </Card>
        <Card title="アイドルと画面ロック">
          <label className="field">
            <span>減光するまでの時間（分、0 で無効）</span>
            <input
              type="number"
              min={0}
              max={1440}
              value={s.idleMinutes}
              onChange={(e) => saveSettings({ ...s, idleMinutes: Number(e.target.value) })}
            />
          </label>
          <Slider
            label="アイドル / ロック時の輝度"
            value={s.idleBrightness}
            onChange={(v) => saveSettings({ ...s, idleBrightness: v })}
          />
        </Card>
        <Card title="夜間スケジュール">
          <Toggle
            label="夜間は輝度を下げる"
            checked={s.nightEnabled}
            onChange={(v) => saveSettings({ ...s, nightEnabled: v })}
          />
          <div className="two-columns">
            <label className="field">
              <span>開始</span>
              <input
                type="time"
                value={s.nightStart}
                onChange={(e) => saveSettings({ ...s, nightStart: e.target.value })}
              />
            </label>
            <label className="field">
              <span>終了</span>
              <input
                type="time"
                value={s.nightEnd}
                onChange={(e) => saveSettings({ ...s, nightEnd: e.target.value })}
              />
            </label>
          </div>
          <Slider
            label="夜間の輝度"
            value={s.nightBrightness}
            onChange={(v) => saveSettings({ ...s, nightBrightness: v })}
          />
        </Card>
      </div>
      <Card title={`Keylume ${appVersion}`}>
        <p className="muted">
          Launchkey MK4 61 向けの非公式ライティングコントローラーです。Novation / Focusrite
          とは関係ありません。
        </p>
        <div className="status-line">
          <span>保存先</span>
          <code className="path">{state.storagePath}</code>
        </div>
        <p className="micro">日本語 UI · ローカル保存 · クラウド接続不要</p>
      </Card>
    </div>
  );
}
export function SetupWizard({
  state,
  act,
  saveSettings,
  onClose,
}: ViewProps & { onClose: () => void }) {
  const [step, setStep] = useState(0);
  const titles = [
    'デバイスを準備する',
    'MIDI 環境を確認',
    '光る場所を確かめる',
    'DAW との使い方',
    '入力転送の準備',
    '最初の光を選ぶ',
  ];
  return (
    <Modal title="Keylume へようこそ" onClose={onClose}>
      <div className="setup-progress">
        {titles.map((_, i) => (
          <span key={i} className={i <= step ? 'active' : ''} />
        ))}
      </div>
      <span className="eyebrow">Step {step + 1} / 6</span>
      <h2>{titles[step]}</h2>
      {step === 0 && (
        <>
          <div className="setup-art">
            <Usb size={45} />
            <span>Launchkey MK4 61</span>
          </div>
          <p>
            USB
            で接続すると、ライティングをコントロールできます。手元に実機がなくてもプレビューで始められます。
          </p>
          <Toggle
            label="プレビューで始める"
            checked={state.settings.mock}
            onChange={(v) => saveSettings({ ...state.settings, mock: v })}
          />
          <p className="micro">{state.status.deviceName}</p>
        </>
      )}
      {step === 1 && (
        <>
          <Note>
            Windows MIDI Services は{' '}
            {state.status.service === 'running'
              ? '稼働しています'
              : 'デスクトップ環境で確認してください'}
            。DAW と同時に使うには、対応する WinMM のマルチクライアント環境が必要です。
          </Note>
          <button className="secondary" onClick={() => void openLink(serviceUrl)}>
            Microsoft の案内を開く
            <ArrowUpRight size={14} />
          </button>
        </>
      )}
      {step === 2 && (
        <>
          <div className="setup-art">
            <Monitor size={45} />
          </div>
          <p>
            LED のアドレスや単色ボタンの種類は、実機によって検証が必要です。「デバイス」画面の LED
            マップ検証から、1 つずつ確認して保存できます。
          </p>
          <Note>
            このセットアップでは後回しにできます。プレビューの結果は実機検証済みとして保存されません。
          </Note>
        </>
      )}
      {step === 3 && (
        <>
          <p>
            Keylume のライティングを使う場合は「ライティング優先」。DAW
            本来のクリップ表示などを使う場合は「DAW に譲る」を選びます。
          </p>
          <label className="field">
            <span>共存モード</span>
            <select
              value={state.settings.coexistMode}
              onChange={(e) => void act('set_coexist_mode', { mode: e.target.value })}
            >
              <option value="lightingFirst">ライティング優先</option>
              <option value="handoff">DAW に譲る</option>
            </select>
          </label>
        </>
      )}
      {step === 4 && (
        <>
          <p>ライティング優先では、ループバックを 2 組作成してパッドとコントロールを転送します。</p>
          <div className="routing-flow">
            <span>Keylume → A</span>
            <ChevronRight size={16} />
            <span>B → DAW</span>
          </div>
          <p>
            DAW では Launchkey の制御サーフェスを解除し、鍵盤の MIDI ポートと B
            側入力を有効にします。
          </p>
          <button className="secondary" onClick={() => void openLink(loopbackUrl)}>
            転送ポートの作成手順
            <ArrowUpRight size={14} />
          </button>
        </>
      )}
      {step === 5 && (
        <>
          <p>プリセットは、いつでも自由に組み替えられます。</p>
          <label className="field">
            <span>最初のプリセット</span>
            <select
              value={state.preset.id}
              onChange={(e) => void act('apply_preset', { id: e.target.value })}
            >
              {state.presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <Note>自動起動は初期状態では OFF です。「設定」画面から有効にできます。</Note>
        </>
      )}
      <div className="modal-actions">
        <button className="subtle" onClick={onClose}>
          あとで設定する
        </button>
        <div>
          {step > 0 && (
            <button className="secondary" onClick={() => setStep(step - 1)}>
              戻る
            </button>
          )}
          <button
            className="primary"
            onClick={() => {
              if (step < 5) setStep(step + 1);
              else {
                saveSettings({ ...state.settings, setupComplete: true });
                onClose();
              }
            }}
          >
            {step === 5 ? 'ライティングを始める' : '次へ'}
            <ChevronRight size={16} />
          </button>
        </div>
      </div>
    </Modal>
  );
}
