import { test, expect } from '@playwright/test';

test('calibrates the closest line and saves transparent effects-only layers', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await page.getByRole('button', { name: '鍵盤の位置合わせ', exact: true }).click();
  const canvas = page.getByLabel('演奏ノート表示');
  const rect = (await canvas.boundingBox())!;
  await page.mouse.move(rect.x + rect.width * 0.05, rect.y + rect.height * 0.81);
  await page.mouse.down();
  await page.mouse.move(rect.x + rect.width * 0.15, rect.y + rect.height * 0.81, { steps: 12 });
  await page.mouse.up();
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('keylume-stage') || '{}').left))
    .toBeCloseTo(0.15, 2);
  await page.getByRole('button', { name: '鍵盤の位置合わせ', exact: true }).click();
  await page.getByLabel('背景を透明にする', { exact: true }).check();
  await page.getByRole('button', { name: 'エフェクトだけにする', exact: true }).click();
  await page.getByLabel('音名の表記', { exact: true }).selectOption('solfege');
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('keylume-stage') || '{}')))
    .toMatchObject({
      transparent: true,
      showBars: false,
      showKeyboard: false,
      showHud: false,
      labelFormat: 'solfege',
      style: 'sparks',
    });
  await expect
    .poll(() =>
      canvas.evaluate(
        (c) => (c as HTMLCanvasElement).getContext('2d')!.getImageData(0, 0, 1, 1).data[3],
      ),
    )
    .toBe(0);
  await page.evaluate(async () => {
    const api = await import('/src/stage/api.ts');
    [60, 61, 64, 67].forEach((note) => api.previewStageInput('keyboard', [144, note, 110]));
  });
  await expect
    .poll(() =>
      canvas.evaluate(
        (c) =>
          Array.from(
            (c as HTMLCanvasElement)
              .getContext('2d')!
              .getImageData(0, 0, (c as HTMLCanvasElement).width, (c as HTMLCanvasElement).height)
              .data,
          ).filter((v, i) => i % 4 === 3 && v > 0).length,
      ),
    )
    .toBeGreaterThan(100);
  for (const style of ['flame', 'aurora', 'rings', 'laser', 'snow', 'rainbow'])
    await page.getByLabel('ノートのスタイル', { exact: true }).selectOption(style);
  await page.screenshot({ path: 'artifacts/v050/stage-effects.png', fullPage: true });
  await page.reload();
  await page.getByRole('button', { name: '演奏', exact: true }).click();
  await expect(page.getByLabel('背景を透明にする', { exact: true })).toBeChecked();
  await expect(page.getByLabel('ノートのバー', { exact: true })).not.toBeChecked();
  expect(errors).toEqual([]);
});
