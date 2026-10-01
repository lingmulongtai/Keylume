import { test, expect } from '@playwright/test';
import { createRequire } from 'node:module';
const { Midi } = createRequire(import.meta.url)('@tonejs/midi');
test('persists hand colors, accompaniment, listening and independent audio timing on both surfaces', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await page.getByRole('button', { name: 'MIDI練習', exact: true }).click();
  const midi = new Midi();
  const left = midi.addTrack();
  left.name = 'Left Hand';
  left.addNote({ midi: 48, time: 1, duration: 1 });
  const right = midi.addTrack();
  right.name = 'Right Hand';
  right.addNote({ midi: 72, time: 1, duration: 1 });
  await page
    .getByLabel('MIDIファイル', { exact: true })
    .setInputFiles({
      name: 'hands.mid',
      mimeType: 'audio/midi',
      buffer: Buffer.from(midi.toArray()),
    });
  await expect(page.getByText('hands', { exact: true })).toBeVisible();
  await page.getByLabel('練習する手', { exact: true }).selectOption('right');
  await page.getByLabel('左手の色', { exact: true }).fill('#ff8800');
  await page.getByLabel('右手の色', { exact: true }).fill('#0088ff');
  await page.getByLabel('MIDI再生音量', { exact: true }).fill('0.25');
  await page.getByLabel('MIDI音声タイミング', { exact: true }).fill('120');
  await page.getByLabel('入力遅延の補正（ms）').fill('-50');
  await page.getByLabel('練習の進め方', { exact: true }).selectOption('listen');
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('keylume-stage')!)))
    .toMatchObject({
      practiceHand: 'right',
      practiceMode: 'listen',
      playbackMode: 'accompaniment',
      playbackVolume: 0.25,
      audioOffsetMs: 120,
      latencyMs: -50,
      leftColor: '#ff8800',
      rightColor: '#0088ff',
    });
  await page.goto('/?view=stage-controls');
  await expect(page.getByLabel('練習する手', { exact: true })).toHaveValue('right');
  await expect(page.getByLabel('MIDI音声タイミング', { exact: true })).toHaveValue('120');
  await page.getByLabel('MIDIの音', { exact: true }).selectOption('off');
  await expect
    .poll(() =>
      page.evaluate(() => JSON.parse(localStorage.getItem('keylume-stage')!).playbackMode),
    )
    .toBe('off');
});
