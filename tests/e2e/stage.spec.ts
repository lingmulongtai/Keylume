import { test, expect } from '@playwright/test';
import { createRequire } from 'node:module';
const { Midi } = createRequire(import.meta.url)('@tonejs/midi');
test('imports MIDI in a worker, controls practice and saves calibrated appearance', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await expect(page.getByRole('heading', { name: '演奏', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'MIDI練習', exact: true }).click();
  const midi = new Midi();
  midi.header.setTempo(100);
  midi
    .addTrack()
    .addNote({ midi: 60, time: 0, duration: 1, velocity: 0.8 })
    .addNote({ midi: 64, time: 1, duration: 1, velocity: 0.8 });
  await page
    .getByLabel('MIDIファイル', { exact: true })
    .setInputFiles({
      name: 'practice.mid',
      mimeType: 'audio/midi',
      buffer: Buffer.from(midi.toArray()),
    });
  await expect(page.getByText('practice', { exact: true })).toBeVisible();
  await expect(page.getByLabel('練習の進め方', { exact: true })).toHaveValue('timing');
  await page.getByRole('button', { name: '練習を開始', exact: true }).click();
  await expect(page.getByRole('button', { name: '練習を一時停止', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '練習を一時停止', exact: true }).click();
  await page.getByLabel('練習速度', { exact: true }).selectOption('0.5');
  await page.getByLabel('ノートのスタイル', { exact: true }).selectOption('particles');
  await page.getByLabel('鍵盤の左端', { exact: true }).fill('0.2');
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('keylume-stage') || '{}')))
    .toMatchObject({ left: 0.2, style: 'particles', speed: 0.5 });
  await page.getByText('パッドドラムとルーパー', { exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'ドラムとルーパー' }).getByRole('button', { name: /Kick/ }),
  ).toBeVisible();
  await page.screenshot({ path: 'artifacts/v040-stage-browser.png', fullPage: true });
  await page.reload();
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await expect(page.getByLabel('鍵盤の左端', { exact: true })).toHaveValue('0.2');
  expect(errors).toEqual([]);
});
