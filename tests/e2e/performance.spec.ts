import { test, expect } from '@playwright/test';
test('live hardware controls, pedal and snapshots survive navigating the editor', async ({
  page,
}) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'ライティング', exact: true })).toBeVisible();
  const fader = page.locator('[data-control="fader-1"]');
  await expect(fader).toHaveAttribute('data-value', 'unknown');
  await page.evaluate(async () => {
    // Exercise the same browser command boundary as the diagnostic screen.
    const api = await import('/src/api.ts');
    for (const [source, bytes] of [
      ['daw', [191, 5, 127]],
      ['daw', [191, 21, 96]],
      ['keyboard', [224, 0, 64]],
      ['keyboard', [176, 64, 127]],
      ['keyboard', [144, 60, 100]],
    ])
      await api.command('simulate_input', { source, bytes });
  });
  await expect(fader).toHaveAttribute('data-value', '127');
  await expect(page.locator('[data-control="encoder-1"]')).toHaveAttribute('data-value', '96');
  await expect(page.locator('[data-control="pitch-wheel"]')).toHaveAttribute('data-value', '8192');
  await expect(page.getByTestId('sustain-state')).toHaveText('Sustain ON');
  await page.getByRole('button', { name: '設定', exact: true }).click();
  await page.getByRole('button', { name: 'ライティング', exact: true }).click();
  await expect(fader).toHaveAttribute('data-value', '127');
  await expect(page.locator('[data-key="60"]')).toHaveAttribute('fill', '#e8c48e');
  await page.getByRole('button', { name: '一時停止', exact: true }).click();
  await expect(fader).toHaveAttribute('data-value', 'unknown');
  await expect(page.getByTestId('sustain-state')).toHaveText('Sustain —');
});
