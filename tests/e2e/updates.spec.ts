import { test, expect } from '@playwright/test';

test('announces a release, opens it on request, dismisses it and exposes check errors', async ({
  page,
}) => {
  await page.goto('/tests/fixtures/updates.html');
  const banner = page.getByRole('status', { name: 'アップデートのお知らせ' });
  await expect(banner).toContainText('v0.3.0 が利用できます');
  await banner.getByRole('button', { name: '更新ページを開く' }).click();
  await expect(page.getByLabel('opened URL')).toHaveText(
    'https://github.com/lingmulongtai/Keylume/releases/tag/v0.3.0',
  );
  await banner.getByRole('button', { name: 'あとで' }).click();
  await expect(banner).toHaveCount(0);
  await page.getByRole('switch', { name: '更新を自動確認' }).click();
  await expect(page.getByLabel('automatic checks')).toHaveText('false');
  await page.getByRole('button', { name: '今すぐ確認' }).click();
  await expect(page.getByRole('button', { name: '確認中…' })).toBeDisabled();
  await expect(page.getByText('更新の通信がタイムアウトしました', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '今すぐ確認' })).toBeEnabled();
  await expect(page.getByText('新しいバージョンはありません', { exact: true })).toHaveCount(0);
});
