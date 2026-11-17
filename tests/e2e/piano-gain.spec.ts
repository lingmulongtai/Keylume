import { test, expect } from '@playwright/test';
test('piano gain is opt-in and persists without changing volume', async ({ page }) => {
  await page.goto('/');
  await page.locator('.piano-details summary').click();
  const gain = page.getByLabel('音源ゲイン', { exact: true });
  await expect(gain).toHaveValue('0');
  await gain.fill('6');
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(localStorage.getItem('keylume-preview-v1') || 'null')?.settings.piano.gainDb,
      ),
    )
    .toBe(6);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          JSON.parse(localStorage.getItem('keylume-preview-v1') || 'null')?.settings.piano.volume,
      ),
    )
    .toBe(0.5);
  await page.reload();
  await page.locator('.piano-details summary').click();
  await expect(gain).toHaveValue('6');
});
