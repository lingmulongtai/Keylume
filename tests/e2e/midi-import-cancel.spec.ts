import { test, expect } from '@playwright/test';

test('leaving MIDI practice cancels a pending parser without loading a song', async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).__parserStopped = 0;
    (window as any).Worker = class {
      onmessage = null;
      onerror = null;
      onmessageerror = null;
      postMessage() {
        (window as any).__parserStarted = true;
      }
      terminate() {
        (window as any).__parserStopped++;
      }
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await page.getByRole('button', { name: 'MIDI練習', exact: true }).click();
  await page
    .getByLabel('MIDIファイル', { exact: true })
    .setInputFiles({ name: 'pending.mid', mimeType: 'audio/midi', buffer: Buffer.from('test') });
  await expect.poll(() => page.evaluate(() => (window as any).__parserStarted)).toBe(true);
  await page.getByRole('button', { name: 'ライブ表示', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__parserStopped)).toBe(1);
  await page.getByRole('button', { name: 'MIDI練習', exact: true }).click();
  await expect(page.getByRole('button', { name: 'MIDIを読み込む', exact: true })).toBeEnabled();
});
