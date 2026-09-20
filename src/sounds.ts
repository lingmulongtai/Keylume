import { invoke } from '@tauri-apps/api/core';
import { native } from './api';
import catalog from '../resources/sound-library.json';
import { pianoSounds } from './piano-sounds';
export interface SoundEntry {
  id: string;
  name: string;
  category: string;
  bank: number;
  program: number;
  installed: boolean;
}
export interface LibraryState {
  entries: SoundEntry[];
  progress: { active: boolean; received: number; total: number; error: string };
}
export const factorySounds: SoundEntry[] = [
  ...pianoSounds.map((s) => ({ ...s, category: 'Piano', bank: 0, program: 0, installed: true })),
  ...catalog.map((s) => ({ ...s, installed: false })),
];
let known: SoundEntry[] = factorySounds;
export const soundName = (id: string) => known.find((s) => s.id === id)?.name ?? '持ち込み音源';
export async function libraryCommand<T = LibraryState>(
  name: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) {
    const result = await invoke<T>('library_command', { name, args });
    if (name === 'state') known = (result as LibraryState).entries;
    return result;
  }
  if (name === 'state')
    return {
      entries: factorySounds,
      progress: { active: false, received: 0, total: 32319396, error: '' },
    } as T;
  throw Error('音源の取得・インポートはWindows版で利用できます');
}
export const categoryNames: Record<string, string> = {
  Piano: 'ピアノ',
  Mallets: 'ベル・マレット',
  Organ: 'オルガン',
  Guitar: 'ギター',
  Bass: 'ベース',
  Strings: 'ストリングス',
  Ensemble: 'アンサンブル',
  Brass: 'ブラス',
  Reed: 'サックス・リード',
  Winds: 'フルート・管楽器',
  'Synth Lead': 'シンセリード',
  'Synth Pad': 'シンセパッド',
  'Synth FX': 'シンセFX',
  World: '民族楽器',
  Percussion: 'パーカッション',
  'Sound FX': 'サウンドFX',
  Imported: '持ち込み',
};
