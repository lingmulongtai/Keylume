import { test, expect } from '@playwright/test';
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'ライティング', exact: true })).toBeVisible();
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
  await expect(page.getByLabel('適用するプリセット').locator('option:checked')).toHaveText(
    'E2E Sunset',
  );
  await page.getByRole('button', { name: 'プリセット', exact: true }).click();
  await page.getByLabel('プリセットを検索', { exact: true }).fill('sunset');
  await expect(page.getByRole('heading', { name: 'E2E Sunset', exact: true })).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'プリセットが見つかりません', exact: true }),
  ).toHaveCount(0);
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
  await page.getByRole('button', { name: '光った（色付き）', exact: true }).click();
  await page.waitForTimeout(300);
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem('keylume-preview-v1')!));
  expect(saved.layout.leds[0].kind).toBe('rgb');
  expect(saved.layout.leds[0].verified).toBe(false);
});
test('validates edited layouts and keeps selection within a smaller layout', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.getByRole('button', { name: 'デバイス', exact: true }).click();
  await page.getByRole('button', { name: '光った（白のみ）', exact: true }).click();
  await expect(page.locator('.toast')).toContainText('LED のアドレスがありません');
  await page.getByRole('button', { name: '次の LED', exact: true }).click();
  await page.getByRole('button', { name: 'レイアウト JSON を編集', exact: true }).click();
  const editor = page.getByLabel('レイアウト JSON', { exact: true });
  const original = JSON.parse(await editor.inputValue());
  await editor.fill(JSON.stringify({ ...original, leds: [] }));
  await page.getByRole('button', { name: 'レイアウトを保存', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.locator('.toast')).toContainText('レイアウトのサイズが不正です');
  await editor.fill(JSON.stringify({ ...original, leds: [original.leds[0]] }));
  await page.getByRole('button', { name: 'レイアウトを保存', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByLabel('pad.top.1 sysexId', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '次の LED', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: '前の LED', exact: true })).toBeDisabled();
  expect(errors).toEqual([]);
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

test('hardware face and clickable controls stay aligned at full zoom', async ({ page }) => {
  await expect(page.locator('.device-base [data-key]')).toHaveCount(61);
  const physical = await page.locator('.device-base [data-control="pad.top.1"] rect').boundingBox();
  const hit = await page
    .getByRole('button', { name: 'pad.top.1 rgb LED', exact: true })
    .boundingBox();
  expect(Math.abs(physical!.x + physical!.width / 2 - hit!.x - hit!.width / 2)).toBeLessThan(1);
  for (let i = 0; i < 10; i++)
    await page.getByRole('button', { name: '拡大', exact: true }).click();
  await page.getByRole('button', { name: 'btn.record mono LED', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'btn.record mono LED', exact: true }),
  ).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('button', { name: '表示をリセット', exact: true }).click();
  await page.getByRole('button', { name: 'プリセット', exact: true }).click();
  await expect(page.locator('.preset-card').first().locator('[data-key]')).toHaveCount(61);
});
