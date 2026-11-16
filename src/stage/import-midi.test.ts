import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { importMidi } from './import-midi';
class FakeWorker {
  static instances: FakeWorker[] = [];
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onerror: (() => void) | null = null;
  onmessageerror: (() => void) | null = null;
  terminate = vi.fn();
  postMessage = vi.fn();
  constructor() {
    FakeWorker.instances.push(this);
  }
}
const file = () =>
  ({ size: 4, name: 'test.mid', arrayBuffer: async () => new ArrayBuffer(4) }) as File;
beforeEach(() => {
  vi.useFakeTimers();
  FakeWorker.instances = [];
  vi.stubGlobal('Worker', FakeWorker);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
const worker = () => FakeWorker.instances[0];
test('success releases worker and timeout', async () => {
  const result = importMidi(file());
  await Promise.resolve();
  const song = { title: 'test' };
  worker().onmessage?.({ data: { song } });
  await expect(result).resolves.toEqual(song);
  expect(worker().terminate).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});
test('abort stops parsing and releases worker', async () => {
  const c = new AbortController();
  const result = importMidi(file(), c.signal);
  const rejected = expect(result).rejects.toMatchObject({ name: 'AbortError' });
  await Promise.resolve();
  c.abort();
  await rejected;
  expect(worker().terminate).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});
test('already cancelled imports do not read file or create a worker', async () => {
  const c = new AbortController();
  c.abort();
  const input = file();
  input.arrayBuffer = vi.fn();
  await expect(importMidi(input, c.signal)).rejects.toMatchObject({ name: 'AbortError' });
  expect(input.arrayBuffer).not.toHaveBeenCalled();
  expect(FakeWorker.instances).toHaveLength(0);
});
test('cancel during file read does not create a worker', async () => {
  const c = new AbortController();
  const result = importMidi(file(), c.signal);
  c.abort();
  await expect(result).rejects.toMatchObject({ name: 'AbortError' });
  expect(FakeWorker.instances).toHaveLength(0);
});
test.each(['onerror', 'onmessageerror'] as const)('%s releases worker', async (event) => {
  const result = importMidi(file());
  const rejected = expect(result).rejects.toThrow();
  await Promise.resolve();
  worker()[event]?.();
  await rejected;
  expect(worker().terminate).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});
test('timeout releases worker', async () => {
  const result = importMidi(file());
  const rejected = expect(result).rejects.toThrow();
  await vi.advanceTimersByTimeAsync(10000);
  await rejected;
  expect(worker().terminate).toHaveBeenCalledOnce();
});
test('postMessage failure releases worker immediately', async () => {
  class BrokenWorker extends FakeWorker {
    postMessage = vi.fn(() => {
      throw Error('transfer failed');
    });
  }
  vi.stubGlobal('Worker', BrokenWorker);
  await expect(importMidi(file())).rejects.toThrow('transfer failed');
  expect(worker().terminate).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});
