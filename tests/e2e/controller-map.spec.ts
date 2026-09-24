import { test, expect } from '@playwright/test';
test('hardware diagram selects a binding and keeps the OLED preference', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'コントローラー', exact: true }).click();
  await page.getByRole('button', { name: '割り当て: 中央 ←', exact: true }).click();
  await expect(page.getByLabel('割り当てる機能', { exact: true })).toHaveValue('lighting');
  await page.getByRole('button', { name: '割り当て: フェーダー 1', exact: true }).click();
  await page.getByLabel('割り当てる機能', { exact: true }).selectOption('tempo');
  await page.getByRole('button', { name: '割り当てを保存', exact: true }).click();
  await page.getByLabel('OLEDの待機表示', { exact: true }).selectOption('preset');
  await expect
    .poll(() =>
      page.evaluate(
        () => JSON.parse(localStorage.getItem('keylume-preview-v1')!).settings.controller,
      ),
    )
    .toMatchObject({ performance: { 'fader-1': { action: 'tempo' } }, displayIdle: 'preset' });
  await page.getByRole('button', { name: '本体設定: Settings', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: '本体側の機能' })).toBeVisible();
  await page.screenshot({ path: 'artifacts/control-stage/controller-map.png', fullPage: true });
});
