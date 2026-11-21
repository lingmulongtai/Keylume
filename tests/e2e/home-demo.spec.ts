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
  const dialog = page.getByRole('dialog', { name: 'MIDIの曲名' });
  const title = dialog.getByRole('textbox', { name: '曲名' });
  await expect(title).toHaveValue('lantern-waltz');
  await title.press('Escape');
  const selection = page.getByLabel('試聴する曲', { exact: true });
  await expect(selection).toHaveValue('pocket-swing');
  await page.getByLabel('試聴するMIDIファイル').setInputFiles('public/demos/lantern-waltz.mid');
  await title.fill('   ');
  await expect(dialog.getByRole('button', { name: '読み込む' })).toBeDisabled();
  await title.fill('  私のワルツ  ');
  await dialog.getByRole('button', { name: '読み込む' }).click();
  await expect(selection).toHaveValue('imported');
  await expect(selection.locator('option:checked')).toHaveText('私のワルツ');
  await player.getByRole('button', { name: '試聴する' }).click();
  await expect(keys.first()).toBeVisible();
  await player.getByRole('button', { name: '試聴を停止' }).click();
  await expect(keys).toHaveCount(0);
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await expect(page.getByLabel('演奏の再生操作')).toContainText('私のワルツ');
});
