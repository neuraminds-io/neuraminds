import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
  test('navigates to markets page via header link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'Markets' }).click();
    await expect(page).toHaveURL('/markets');
    await expect(page.getByRole('heading', { name: /all markets/i })).toBeVisible();
  });

  test('navigates to portfolio page via header link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'Portfolio' }).click();
    await expect(page).toHaveURL('/portfolio');
  });

  test('navigates home via logo click', async ({ page }) => {
    await page.goto('/markets');
    await page.getByRole('link', { name: /polyguard/i }).click();
    await expect(page).toHaveURL('/');
  });

  test('navigates to leaderboard page', async ({ page }) => {
    await page.goto('/leaderboard');
    await expect(page).toHaveURL('/leaderboard');
  });

  test('navigates to settings page', async ({ page }) => {
    await page.goto('/settings');
    await expect(page).toHaveURL('/settings');
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  });

  test('navigates to profile page with wallet address', async ({ page }) => {
    const testWallet = '7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU';
    await page.goto(`/profile/${testWallet}`);
    await expect(page).toHaveURL(`/profile/${testWallet}`);
  });
});

test.describe('Header Active States', () => {
  test('markets link shows active state on markets page', async ({ page }) => {
    await page.goto('/markets');
    const marketsLink = page.getByRole('link', { name: 'Markets' });
    await expect(marketsLink).toHaveClass(/bg-bg-secondary/);
  });

  test('portfolio link shows active state on portfolio page', async ({ page }) => {
    await page.goto('/portfolio');
    const portfolioLink = page.getByRole('link', { name: 'Portfolio' });
    await expect(portfolioLink).toHaveClass(/bg-bg-secondary/);
  });
});
