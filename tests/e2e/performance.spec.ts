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
    const url = performance
      .getEntriesByType('resource')
      .find((entry) => /\/src\/api\.ts(?:\?|$)/.test(entry.name))!.name;
    const api = await import(url);
    for (const [source, bytes] of [
      ['daw', [191, 5, 127]],
      ['daw', [191, 21, 96]],
      ['daw', [182, 73, 1]],
      ['daw', [190, 5, 127]],
      ['keyboard', [224, 0, 64]],
      ['keyboard', [176, 64, 127]],
      ['keyboard', [144, 60, 100]],
    ])
      await api.command('simulate_input', { source, bytes });
  });
  await expect(fader).toHaveAttribute('data-value', '127');
  await expect(page.locator('[data-control=arp]')).toHaveAttribute('data-active', '1');
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

test('piano preferences persist and screen keys release on focus loss', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'ピアノをオン', exact: true }).click();
  await page.getByRole('button', { name: 'ピアノを1オクターブ上げる' }).click();
  await expect(page.getByLabel('ピアノのオクターブ', { exact: true })).toHaveText('+1');
  await page.getByLabel('ピアノ音量', { exact: true }).fill('0.3');
  await page.getByText('音声と演奏の設定', { exact: true }).click();
  await page.getByLabel('音声バッファ', { exact: true }).selectOption('512');
  await expect
    .poll(async () =>
      page.evaluate(() => JSON.parse(localStorage.getItem('keylume-preview-v1')!)?.settings.piano),
    )
    .toMatchObject({ enabled: true, octave: 1, volume: 0.3, bufferFrames: 512 });
  await page.reload();
  await expect(page.getByRole('button', { name: 'ピアノをオフ', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(page.getByLabel('ピアノのオクターブ', { exact: true })).toHaveText('+1');
  const key = page.getByRole('button', { name: '鍵盤 60（ピアノ）', exact: true });
  await key.focus();
  await page.keyboard.down('Space');
  await expect(page.locator('[data-key="60"]')).toHaveAttribute('fill', '#e8c48e');
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await expect(page.locator('[data-key="60"]')).not.toHaveAttribute('fill', '#e8c48e');
  await page.keyboard.up('Space');
  await page.getByRole('button', { name: '全音停止', exact: true }).click();
});

test('lighting pause and resume retain playing keys and pedal with piano enabled', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'ピアノをオン', exact: true }).click();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType('resource')
      .find((entry) => /\/src\/api\.ts(?:\?|$)/.test(entry.name))!.name;
    const api = await import(url);
    for (const [source, bytes] of [
      ['keyboard', [144, 60, 100]],
      ['keyboard', [176, 64, 127]],
      ['daw', [191, 5, 100]],
      ['daw', [191, 115, 127]],
    ])
      await api.command('simulate_input', { source, bytes });
  });
  await expect(page.getByTestId('sustain-state')).toHaveText('Sustain ON');
  await page.getByRole('button', { name: '一時停止', exact: true }).click();
  await expect(page.locator('[data-control="fader-1"]')).toHaveAttribute('data-value', 'unknown');
  await expect(page.locator('[data-key="60"]')).toHaveAttribute('fill', '#e8c48e');
  await expect(page.getByTestId('sustain-state')).toHaveText('Sustain ON');
  await page.getByRole('button', { name: '再開', exact: true }).click();
  await expect(page.locator('[data-key="60"]')).toHaveAttribute('fill', '#e8c48e');
  await expect(page.getByTestId('sustain-state')).toHaveText('Sustain ON');
});
