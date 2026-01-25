import { defineConfig, devices } from '@playwright/test';

/**
 * Serena E2E Test Configuration
 *
 * Test Categories:
 * - @frontend: WASM Dashboard UI tests
 * - @integration: Frontend-Backend integration tests
 * - @performance: MCP server performance benchmarks
 * - @stability: Long-running stability tests
 */
export default defineConfig({
  testDir: './tests',

  // Run tests in parallel for faster execution
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry failed tests in CI
  retries: process.env.CI ? 2 : 0,

  // Limit workers to reduce flakiness
  workers: process.env.CI ? 1 : undefined,

  // Reporter configuration
  reporter: [
    ['html', { outputFolder: 'test-results/html-report' }],
    ['json', { outputFile: 'test-results/results.json' }],
    ['list'],
  ],

  // Global timeout
  timeout: 30000,

  // Expect timeout
  expect: {
    timeout: 5000,
  },

  use: {
    // Base URL for the dashboard
    baseURL: process.env.DASHBOARD_URL || 'http://localhost:8080',

    // Capture trace on first retry
    trace: 'on-first-retry',

    // Capture screenshots on failure
    screenshot: 'only-on-failure',

    // Video recording
    video: 'on-first-retry',
  },

  // Test projects for different configurations
  projects: [
    // Frontend tests in Chromium
    {
      name: 'frontend-chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
      testMatch: /.*frontend.*\.spec\.ts/,
    },

    // Frontend tests in Firefox
    {
      name: 'frontend-firefox',
      use: {
        ...devices['Desktop Firefox'],
      },
      testMatch: /.*frontend.*\.spec\.ts/,
    },

    // Integration tests (API + UI)
    {
      name: 'integration',
      use: {
        ...devices['Desktop Chrome'],
      },
      testMatch: /.*integration.*\.spec\.ts/,
    },

    // Performance tests
    {
      name: 'performance',
      use: {
        ...devices['Desktop Chrome'],
      },
      testMatch: /.*performance.*\.spec\.ts/,
      timeout: 120000, // Longer timeout for perf tests
    },

    // Stability tests
    {
      name: 'stability',
      use: {
        ...devices['Desktop Chrome'],
      },
      testMatch: /.*stability.*\.spec\.ts/,
      timeout: 300000, // 5 minutes for stability tests
    },
  ],

  // Web server configuration for dashboard
  webServer: [
    {
      // Start the WASM dashboard via trunk
      command: 'cd ../crates/serena-dashboard && trunk serve --port 8080',
      url: 'http://localhost:8080',
      reuseExistingServer: !process.env.CI,
      timeout: 120000,
    },
    {
      // Start the MCP server (web mode for SSE)
      command: 'cargo run --package serena -- start --transport web --port 3000',
      url: 'http://localhost:3000/health',
      reuseExistingServer: !process.env.CI,
      timeout: 120000,
      cwd: '..',
    },
  ],
});
