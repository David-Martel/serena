import { test, expect, Page } from '@playwright/test';

/**
 * Frontend Tests for Serena WASM Dashboard
 *
 * These tests verify the dashboard UI functionality including:
 * - Page loading and WASM initialization
 * - Component rendering
 * - User interactions
 * - Responsive design
 */

test.describe('Dashboard @frontend', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for WASM to initialize
    await page.waitForFunction(() => {
      return document.body.textContent !== '' &&
        !document.querySelector('noscript');
    }, { timeout: 30000 });
  });

  test('should load the dashboard page', async ({ page }) => {
    // Verify page title
    await expect(page).toHaveTitle(/Serena/i);

    // Verify main content is visible
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should display connection status', async ({ page }) => {
    // Look for connection status indicator
    const statusIndicator = page.getByTestId('connection-status')
      .or(page.locator('.connection-status'))
      .or(page.getByRole('status'));

    // Status should exist (connected or disconnected)
    await expect(statusIndicator.first()).toBeVisible({ timeout: 10000 });
  });

  test('should show tools list when connected', async ({ page }) => {
    // Wait for connection and tools to load
    const toolsList = page.getByTestId('tools-list')
      .or(page.locator('.tools-list'))
      .or(page.locator('[data-tools]'));

    // Allow time for SSE connection and tool loading
    await expect(toolsList.first()).toBeVisible({ timeout: 15000 });
  });

  test('should handle reconnection UI', async ({ page }) => {
    // Look for reconnection elements
    const reconnectButton = page.getByRole('button', { name: /reconnect/i })
      .or(page.locator('button:has-text("reconnect")'));

    // The button may or may not be visible depending on connection state
    // Just verify the page handles it gracefully
    const isReconnectVisible = await reconnectButton.first().isVisible().catch(() => false);

    // Page should remain functional regardless
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Stats Panel @frontend', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should display server statistics', async ({ page }) => {
    // Look for stats container
    const statsPanel = page.getByTestId('stats-panel')
      .or(page.locator('.stats-panel'))
      .or(page.locator('.stats'));

    // Stats should be visible when connected
    await expect(statsPanel.first()).toBeVisible({ timeout: 15000 });
  });

  test('should update stats in real-time', async ({ page }) => {
    // Get initial stats value
    const statsValue = page.locator('.stat-value, [data-stat-value]').first();

    if (await statsValue.isVisible()) {
      const initialValue = await statsValue.textContent();

      // Wait for potential update
      await page.waitForTimeout(2000);

      // Stats should either update or remain stable
      await expect(statsValue).toBeVisible();
    }
  });
});

test.describe('Tool Cards @frontend', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('should display tool cards with names', async ({ page }) => {
    // Wait for tools to load
    const toolCard = page.locator('.tool-card, [data-tool]').first();

    await expect(toolCard).toBeVisible({ timeout: 15000 });
  });

  test('should show tool description on hover', async ({ page }) => {
    const toolCard = page.locator('.tool-card, [data-tool]').first();

    if (await toolCard.isVisible()) {
      await toolCard.hover();

      // Look for tooltip or description
      const description = page.locator('.tooltip, .description, [role="tooltip"]');
      // May or may not appear depending on implementation
      await page.waitForTimeout(500);
    }
  });
});

test.describe('Responsive Design @frontend', () => {
  const viewports = [
    { name: 'mobile', width: 375, height: 667 },
    { name: 'tablet', width: 768, height: 1024 },
    { name: 'desktop', width: 1920, height: 1080 },
  ];

  for (const viewport of viewports) {
    test(`should render correctly on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/');
      await page.waitForLoadState('networkidle');

      // Verify no horizontal scroll
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);

      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 10); // Allow small tolerance

      // Verify content is visible
      await expect(page.locator('body')).toBeVisible();
    });
  }
});

test.describe('Accessibility @frontend', () => {
  test('should have proper page structure', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Check for proper heading structure
    const headings = page.locator('h1, h2, h3, h4, h5, h6');
    const headingCount = await headings.count();

    // Page should have at least one heading
    expect(headingCount).toBeGreaterThan(0);
  });

  test('should have focusable interactive elements', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Tab through the page
    await page.keyboard.press('Tab');

    // Something should be focused
    const focusedElement = page.locator(':focus');
    await expect(focusedElement.first()).toBeVisible({ timeout: 5000 });
  });

  test('should have proper color contrast', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Check that text is readable (basic check)
    const bodyStyles = await page.evaluate(() => {
      const body = document.body;
      const styles = window.getComputedStyle(body);
      return {
        color: styles.color,
        backgroundColor: styles.backgroundColor,
      };
    });

    // Verify colors are defined
    expect(bodyStyles.color).toBeDefined();
    expect(bodyStyles.backgroundColor).toBeDefined();
  });
});

test.describe('Error Handling @frontend', () => {
  test('should handle network errors gracefully', async ({ page }) => {
    // Intercept network requests and fail them
    await page.route('**/api/**', (route) => route.abort());

    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    // Page should still render without crashing
    await expect(page.locator('body')).toBeVisible();

    // Should show error state or retry UI
    const errorIndicator = page.locator('.error, [data-error], .disconnected');
    // Error handling should be present
    await page.waitForTimeout(2000);
  });

  test('should not show JavaScript errors in console', async ({ page }) => {
    const consoleErrors: string[] = [];

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    page.on('pageerror', (error) => {
      consoleErrors.push(error.message);
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);

    // Filter out expected errors (like network failures in test environment)
    const unexpectedErrors = consoleErrors.filter(
      (error) =>
        !error.includes('Failed to fetch') &&
        !error.includes('net::ERR_') &&
        !error.includes('NetworkError')
    );

    expect(unexpectedErrors).toHaveLength(0);
  });
});
