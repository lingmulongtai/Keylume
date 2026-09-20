import { test, expect } from '@playwright/test';
test('sound cards filter by family and favorites while drum edits survive kit changes', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: '音源を選ぶ', exact: true }).click();
  await page.getByRole('button', { name: 'シンセパッド', exact: true }).click();
  await expect(page.locator('.sound-card').first()).toContainText('初回ダウンロード');
  await expect(page.locator('.sound-card').first()).toBeVisible();
  await page.getByRole('button', { name: '使用可能', exact: true }).click();
  await expect(page.locator('.sound-card')).toHaveCount(4);
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await page.getByText('パッドドラムとルーパー', { exact: true }).click();
  await page.getByLabel('ドラムキット', { exact: true }).selectOption('2');
  await page.getByText('パッドの音作り', { exact: true }).click();
  await page.getByLabel('パッドの音の種類', { exact: true }).selectOption('12');
  await page.getByLabel('パッドの定位', { exact: true }).fill('-0.5');
  await page.getByLabel('ドラムキット', { exact: true }).selectOption('1');
  await expect(page.getByLabel('パッドの音の種類', { exact: true })).toHaveValue('8');
  await page.getByLabel('ドラムキット', { exact: true }).selectOption('2');
  await expect(page.getByLabel('パッドの音の種類', { exact: true })).toHaveValue('12');
  await expect(page.getByLabel('パッドの定位', { exact: true })).toHaveValue('-0.5');
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(localStorage.getItem('keylume-preview-v1')!)?.settings.piano.drumKit
            .banks[2][8].pan,
      ),
    )
    .toBe(-0.5);
  await page.reload();
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await page.getByText('パッドドラムとルーパー', { exact: true }).click();
  await expect(page.getByLabel('ドラムキット', { exact: true })).toHaveValue('2');
});
