import { expect, test } from '@playwright/test';

test('Home auditions bundled and imported MIDI on the keyboard and shares the Stage transport', async ({
  page,
}) => {
  await page.goto('/');
  const player = page.getByRole('region', { name: '音源の試聴' });
  const keys = page.locator('.device-interaction [aria-label^="鍵盤"][aria-pressed="true"]');
  await player.getByRole('button', { name: '試聴する' }).click();
  await expect(keys.first()).toBeVisible();
  await expect(page.getByLabel('ピアノをオフ', { exact: true })).toBeVisible();
  await player.getByRole('button', { name: '一時停止' }).click();
  await expect(keys).toHaveCount(0);
  await page.getByLabel('試聴する曲', { exact: true }).selectOption('pocket-swing');
  await player.getByRole('button', { name: '試聴する' }).click();
  await expect(keys.first()).toBeVisible();
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await expect(page.getByLabel('演奏の再生操作')).toContainText('Pocket Swing');
  await page.getByRole('button', { name: '練習を一時停止' }).click();
  await page.getByRole('button', { name: 'ホーム', exact: true }).click();
  await expect(keys).toHaveCount(0);
  await page.getByLabel('試聴するMIDIファイル').setInputFiles('public/demos/lantern-waltz.mid');
  await expect(page.getByLabel('試聴する曲', { exact: true })).toHaveValue('lantern-waltz');
  await player.getByRole('button', { name: '試聴する' }).click();
  await expect(keys.first()).toBeVisible();
  await player.getByRole('button', { name: '試聴を停止' }).click();
  await expect(keys).toHaveCount(0);
});
