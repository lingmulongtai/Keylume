import type { Song } from './types';
export async function importMidi(file: File): Promise<Song> {
  if (file.size > 16 * 1024 * 1024) throw Error('MIDIは16 MB以下にしてください');
  const bytes = await file.arrayBuffer();
  const worker = new Worker(new URL('./midi-worker.ts', import.meta.url), { type: 'module' });
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      worker.terminate();
      reject(Error('MIDIの解析が制限時間を超えました'));
    }, 10000);
    const cleanup = () => {
      clearTimeout(timeout);
      worker.terminate();
    };
    worker.onmessage = ({ data }) => {
      cleanup();
      if (data.error) reject(Error(data.error));
      else resolve(data.song);
    };
    worker.onerror = () => {
      cleanup();
      reject(Error('MIDIファイルを読み込めませんでした'));
    };
    worker.postMessage({ bytes, title: file.name }, [bytes]);
  });
}
