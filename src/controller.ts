export interface Binding {
  action: string;
  value: string;
}
export interface ControllerSettings {
  enabled: boolean;
  mode: 'performance' | 'desktop';
  performance: Record<string, Binding>;
  desktop: Record<string, Binding>;
  scrollSpeed: number;
}
export const effectNames = [
  'reverb',
  'delay',
  'cutoff',
  'resonance',
  'chorus',
  'drive',
  'width',
  'tremolo',
] as const;
export const effectLabels = [
  'リバーブ',
  'ディレイ',
  'フィルター',
  'レゾナンス',
  'コーラス',
  'ドライブ',
  'ステレオ幅',
  'トレモロ',
];
export const defaultController = (): ControllerSettings => {
  const performance: Record<string, Binding> = {};
  effectNames.forEach(
    (name, i) => (performance[`encoder-${i + 1}`] = { action: 'effect', value: name }),
  );
  for (const [id, action, value] of [
    ['btn.encoderUp', 'sound', '-1'],
    ['btn.encoderDown', 'sound', '1'],
    ['btn.padUp', 'kit', '-1'],
    ['btn.padDown', 'kit', '1'],
    ['btn.trackPrevious', 'lighting', '-1'],
    ['btn.trackNext', 'lighting', '1'],
    ['btn.undo', 'undo', ''],
    ['btn.play', 'loopPlay', ''],
    ['btn.stop', 'loopStop', ''],
    ['btn.record', 'loopRecord', ''],
    ['btn.loop', 'loopOverdub', ''],
    ['fbtn.9', 'mode', ''],
  ])
    performance[id] = { action, value };
  for (let i = 1; i <= 8; i++)
    performance[`fbtn.${i}`] = { action: 'favorite', value: String(i - 1) };
  return {
    enabled: true,
    mode: 'performance',
    scrollSpeed: 1,
    performance,
    desktop: Object.fromEntries(
      [
        ['fbtn.9', 'mode', ''],
        ['btn.undo', 'shortcut', 'Ctrl+Z'],
        ['btn.play', 'shortcut', 'MediaPlayPause'],
        ['btn.stop', 'shortcut', 'MediaStop'],
        ['btn.trackPrevious', 'shortcut', 'MediaPrevious'],
        ['btn.trackNext', 'shortcut', 'MediaNext'],
        ['pitch-wheel', 'scroll', ''],
        ['fbtn.1', 'shortcut', 'VolumeDown'],
        ['fbtn.2', 'shortcut', 'VolumeUp'],
        ['fbtn.3', 'shortcut', 'VolumeMute'],
      ].map(([id, action, value]) => [id, { action, value }]),
    ),
  };
};
export const actionNames: Record<string, string> = {
  none: '割り当てなし',
  mode: '演奏 / デスクトップ切替',
  effect: '音源エフェクト',
  volume: '楽器の音量',
  brightness: 'ライティングの明るさ',
  sound: '音源を切替',
  kit: 'ドラムキットを切替',
  lighting: 'ライティングを切替',
  favorite: 'お気に入り音源',
  piano: '楽器のオン / オフ',
  undo: 'ルーパーを元に戻す',
  loopPlay: 'ループ再生',
  loopStop: 'ループ停止',
  loopRecord: '録音 / 重ね録り',
  loopOverdub: '重ね録り切替',
  loopClear: 'ループを消去',
  panic: '全音停止',
  shortcut: 'キーボード・メディア操作',
  open: 'アプリ / Webページを開く',
  scroll: 'スクロール（中央で停止）',
};
export const controlNames: Record<string, string> = {
  'btn.trackPrevious': '中央 ←',
  'btn.trackNext': '中央 →',
  'btn.encoderUp': 'ノブ横 ↑',
  'btn.encoderDown': 'ノブ横 ↓',
  'btn.padUp': 'パッド横 ↑',
  'btn.padDown': 'パッド横 ↓',
  'pitch-wheel': 'ピッチホイール',
  'mod-wheel': 'モジュレーション',
};
export function bindingName(binding?: Binding) {
  if (!binding || binding.action === 'none') return '割り当てなし';
  if (binding.action === 'effect')
    return effectLabels[effectNames.indexOf(binding.value as (typeof effectNames)[number])];
  return `${actionNames[binding.action]}${binding.value ? ` · ${binding.action === 'favorite' ? Number(binding.value) + 1 : binding.value === '-1' ? '前へ' : binding.value === '1' ? '次へ' : binding.value}` : ''}`;
}
