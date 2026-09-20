import { test, expect } from '@playwright/test';
test('controller keeps independent mode bindings and records keyboard shortcuts', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'コントローラー', exact: true }).click();
  await expect(page.getByLabel('割り当てるエフェクト', { exact: true })).toHaveValue('reverb');
  await page.getByLabel('割り当てるエフェクト', { exact: true }).selectOption('delay');
  await page.getByRole('button', { name: '割り当てを保存', exact: true }).click();
  await expect(
    page.getByRole('status').filter({ hasText: '割り当てを保存しました' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'デスクトップ時の割り当て', exact: true }).click();
  await page.locator('.controller-list button').filter({ hasText: 'Undo' }).click();
  await expect(page.getByLabel('割り当ての内容', { exact: true })).toHaveValue('Ctrl+Z');
  await page.getByRole('button', { name: 'キーを記録', exact: true }).click();
  await page.keyboard.press('Control+Shift+KeyK');
  await expect(page.getByLabel('割り当ての内容', { exact: true })).toHaveValue('Ctrl+Shift+K');
  await page.getByRole('button', { name: '割り当てを保存', exact: true }).click();
  await page.getByRole('button', { name: 'デスクトップモード', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () => JSON.parse(localStorage.getItem('keylume-preview-v1')!)?.settings.controller,
      ),
    )
    .toMatchObject({
      mode: 'desktop',
      performance: { 'encoder-1': { action: 'effect', value: 'delay' } },
      desktop: { 'btn.undo': { action: 'shortcut', value: 'Ctrl+Shift+K' } },
    });
  await page.reload();
  await expect(
    page.getByRole('button', { name: '演奏とデスクトップを切替', exact: true }),
  ).toHaveText('デスクトップモード');
});
