import type { Song } from './types';
export async function importMidi(file: File, signal?: AbortSignal): Promise<Song> {
  signal?.throwIfAborted();
  if (file.size > 16 * 1024 * 1024) throw Error('MIDIは16 MB以下にしてください');
  const bytes = await file.arrayBuffer();
  signal?.throwIfAborted();
  const worker = new Worker(new URL('./midi-worker.ts', import.meta.url), { type: 'module' });
  return new Promise((resolve, reject) => {
    let settled = false;
    const cleanup = () => {
      clearTimeout(timeout);
      signal?.removeEventListener('abort', abort);
      worker.onmessage = null;
      worker.onerror = null;
      worker.onmessageerror = null;
      worker.terminate();
    };
    const fail = (error: unknown) => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(error);
    };
    const abort = () => fail(signal?.reason ?? new DOMException('Aborted', 'AbortError'));
    const timeout = setTimeout(() => fail(Error('MIDIの解析が制限時間を超えました')), 10000);
    worker.onmessage = ({ data }) => {
      if (settled) return;
      if (data?.error) return fail(Error(data.error));
      if (!data?.song) return fail(Error('MIDIファイルを読み込めませんでした'));
      settled = true;
      cleanup();
      resolve(data.song);
    };
    worker.onerror = worker.onmessageerror = () =>
      fail(Error('MIDIファイルを読み込めませんでした'));
    signal?.addEventListener('abort', abort, { once: true });
    if (signal?.aborted) return abort();
    try {
      worker.postMessage({ bytes, title: file.name }, [bytes]);
    } catch (error) {
      fail(error);
    }
  });
}
