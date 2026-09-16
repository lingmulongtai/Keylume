import { test, expect } from '@playwright/test';
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '光に、あなたのリズムを。' })).toBeVisible();
});
test('edits, saves, reloads, exports and deletes a preset', async ({ page }) => {
  await page.getByRole('button', { name: 'レイヤーを追加', exact: true }).click();
  await page.getByRole('button', { name: '単色 static', exact: true }).click();
  await page.getByLabel('エフェクトの色', { exact: true }).fill('#ff3300');
  await page.getByRole('button', { name: 'プリセットを保存', exact: true }).click();
  await page.getByLabel('プリセット名', { exact: true }).fill('E2E Sunset');
  await page.getByRole('button', { name: '保存する', exact: true }).click();
  await expect(page.locator('.toast')).toContainText('保存しました');
  await page.reload();
  await expect(page.getByRole('button', { name: 'E2E Sunset', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'プリセット', exact: true }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'E2E Sunsetを書き出す' }).click();
  expect((await download).suggestedFilename()).toMatch(/\.keylume\.json$/);
  await page.getByRole('button', { name: 'E2E Sunsetを削除' }).click();
  await expect(page.getByRole('heading', { name: 'E2E Sunset', exact: true })).toHaveCount(0);
});
test('pauses, resumes, selects LED zones, and paints', async ({ page }) => {
  await page.getByRole('button', { name: '一時停止', exact: true }).click();
  await expect(page.getByRole('button', { name: '再開', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '再開', exact: true }).click();
  await page.getByRole('button', { name: 'pad.top.1 rgb LED', exact: true }).click();
  await expect(page.getByRole('button', { name: '選択 1', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: '選択 1', exact: true }).click();
  await expect(page.locator('.inspector-effect')).toContainText('カスタムゾーン');
  await page.getByRole('button', { name: 'ペイントモード', exact: true }).click();
  await page.getByRole('button', { name: 'pad.top.2 rgb LED', exact: true }).click();
  await expect(page.locator('.inspector-effect h2')).toHaveText('ペイント');
});
test('creates a profile and exercises mock DAW handoff', async ({ page }) => {
  await page.getByRole('button', { name: 'プロファイル', exact: true }).click();
  await page.getByRole('button', { name: '夜間プロファイルを作る', exact: true }).click();
  await page.getByRole('button', { name: 'プロファイルを保存', exact: true }).click();
  await expect(page.getByRole('heading', { name: '夜の制作', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '共存設定', exact: true }).click();
  await page.locator('.mode-card').filter({ hasText: 'DAW に譲る' }).click();
  await page.getByRole('button', { name: 'デバイス', exact: true }).click();
  await page.getByRole('button', { name: 'DAW 起動 / 終了', exact: true }).click();
  await expect(page.locator('.connection')).toContainText('DAW 使用中');
  await page.getByRole('button', { name: 'DAW 起動 / 終了', exact: true }).click();
  await expect(page.locator('.connection')).toContainText('プレビュー');
});
test('probe never claims hardware verification in mock mode', async ({ page }) => {
  await page.getByRole('button', { name: 'デバイス', exact: true }).click();
  await page.getByRole('button', { name: '点灯テスト', exact: true }).click();
  await expect(page.getByRole('button', { name: 'テストを再実行', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '光った（白のみ）', exact: true }).click();
  await page.waitForTimeout(300);
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem('keylume-preview-v1')!));
  expect(saved.layout.leds[0].kind).toBe('mono');
  expect(saved.layout.leds[0].verified).toBe(false);
});
test('all screens load without browser errors at the minimum window size', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.setViewportSize({ width: 1024, height: 680 });
  for (const name of [
    'プリセット',
    'プロファイル',
    '共存設定',
    'デバイス',
    '設定',
    'ライティング',
  ]) {
    await page.getByRole('button', { name, exact: true }).click();
    await expect(page.locator('main')).not.toBeEmpty();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true);
  }
  await page.getByRole('button', { name: 'セットアップガイド', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(errors).toEqual([]);
});
