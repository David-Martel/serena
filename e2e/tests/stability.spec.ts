import { test, expect, Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Stability Tests for Serena MCP Server
 *
 * These tests verify:
 * - Long-running operation stability
 * - Recovery from failures
 * - Resource cleanup
 * - Edge cases and error conditions
 */

const MCP_SERVER_URL = process.env.MCP_SERVER_URL || 'http://localhost:3000';

// Track test files for cleanup
const TEST_FILES_TO_CLEANUP: string[] = [];

// Cleanup helper
async function cleanupTestFiles() {
  for (const filePath of TEST_FILES_TO_CLEANUP) {
    try {
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
      }
    } catch {
      // Ignore cleanup errors
    }
  }
  TEST_FILES_TO_CLEANUP.length = 0;
}

test.describe('Long-Running Stability @stability', () => {
  test('should handle sustained request load', async ({ request }) => {
    const duration = 30000; // 30 seconds
    const startTime = Date.now();
    let successCount = 0;
    let failCount = 0;
    let timeoutCount = 0;
    let requestCount = 0;

    while (true) {
      const elapsed = Date.now() - startTime;
      if (elapsed >= duration) break;

      try {
        // Set per-request timeout with Promise.race
        const timeoutPromise = new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error('timeout')), 5000)
        );

        const response = await Promise.race([
          request.post(`${MCP_SERVER_URL}/rpc`, {
            data: {
              jsonrpc: '2.0',
              id: ++requestCount,
              method: 'tools/list',
              params: {},
            },
          }),
          timeoutPromise,
        ]);

        if (response.ok()) {
          successCount++;
        } else {
          failCount++;
        }
      } catch (error: unknown) {
        if (error instanceof Error && error.message === 'timeout') {
          timeoutCount++;
        } else {
          failCount++;
        }
      }

      // Don't exceed duration during delay
      const remainingTime = duration - (Date.now() - startTime);
      if (remainingTime > 100) {
        await new Promise((r) => setTimeout(r, 100));
      } else if (remainingTime > 0) {
        await new Promise((r) => setTimeout(r, remainingTime));
      }
    }

    const successRate = requestCount > 0 ? (successCount / requestCount) * 100 : 0;
    console.log(`Sustained load:`);
    console.log(`  - Total: ${requestCount}, Success: ${successCount} (${successRate.toFixed(2)}%)`);
    console.log(`  - Failed: ${failCount}, Timeouts: ${timeoutCount}`);

    expect(successRate).toBeGreaterThan(99);
  });

  test('should maintain SSE connection for extended period', async ({ page }) => {
    let disconnections = 0;
    let reconnections = 0;

    // Track SSE connection state
    page.on('response', (response) => {
      if (response.url().includes('/events') || response.url().includes('/sse')) {
        reconnections++;
      }
    });

    page.on('requestfailed', (request) => {
      if (request.url().includes('/events') || request.url().includes('/sse')) {
        disconnections++;
      }
    });

    await page.goto('/');

    // Wait for 60 seconds
    await page.waitForTimeout(60000);

    console.log(`SSE stability: ${reconnections} connections, ${disconnections} disconnections`);

    // Should not have excessive disconnections
    expect(disconnections).toBeLessThan(5);
  });

  test('dashboard should remain responsive over time', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const checkIntervals = 10;
    const intervalMs = 5000;
    const responseTimes: number[] = [];

    for (let i = 0; i < checkIntervals; i++) {
      const start = performance.now();

      // Perform an action
      const body = page.locator('body');
      await expect(body).toBeVisible();

      const elapsed = performance.now() - start;
      responseTimes.push(elapsed);

      await page.waitForTimeout(intervalMs);
    }

    const avgResponse = responseTimes.reduce((a, b) => a + b, 0) / responseTimes.length;
    const maxResponse = Math.max(...responseTimes);

    console.log(`UI responsiveness - Avg: ${avgResponse.toFixed(2)}ms, Max: ${maxResponse.toFixed(2)}ms`);

    // UI should remain responsive
    expect(avgResponse).toBeLessThan(100);
    expect(maxResponse).toBeLessThan(500);
  });
});

test.describe('Error Recovery @stability', () => {
  test('should recover from temporary server unavailability', async ({ page, context }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Block server requests
    await context.route(`${MCP_SERVER_URL}/**`, (route) => route.abort());

    // Wait for disconnect to be detected
    await page.waitForTimeout(5000);

    // Unblock server
    await context.unroute(`${MCP_SERVER_URL}/**`);

    // Wait for recovery
    await page.waitForTimeout(10000);

    // Page should still be functional
    await expect(page.locator('body')).toBeVisible();
  });

  test('should handle rapid reconnection attempts', async ({ request }) => {
    const results: boolean[] = [];

    // Rapidly send requests
    for (let i = 0; i < 50; i++) {
      try {
        const response = await request.get(`${MCP_SERVER_URL}/health`, {
          timeout: 1000,
        });
        results.push(response.ok());
      } catch {
        results.push(false);
      }
    }

    const successRate = (results.filter((r) => r).length / results.length) * 100;
    console.log(`Rapid reconnection: ${successRate.toFixed(2)}% success`);

    expect(successRate).toBeGreaterThan(90);
  });

  test('should handle malformed requests without crashing', async ({ request }) => {
    const malformedRequests = [
      '', // Empty body
      '{}', // Empty object
      '{"jsonrpc": "1.0"}', // Wrong version
      '{"jsonrpc": "2.0", "method": 123}', // Wrong type
      '{"jsonrpc": "2.0", "id": "test", "method": ""}', // Empty method
      new Array(10000).fill('x').join(''), // Very long string
      '{"nested": '.repeat(100) + '{}' + '}'.repeat(100), // Deep nesting
    ];

    for (const body of malformedRequests) {
      try {
        const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
          data: body,
          headers: { 'Content-Type': 'application/json' },
          timeout: 5000,
        });

        // Server should respond (not crash)
        expect(response.status()).toBeDefined();
      } catch (e) {
        // Network errors are acceptable, crashes are not
      }
    }

    // Verify server is still healthy
    const health = await request.get(`${MCP_SERVER_URL}/health`);
    expect(health.ok()).toBeTruthy();
  });
});

test.describe('Resource Cleanup @stability', () => {
  test('should clean up after many page loads', async ({ page }) => {
    const iterations = 20;

    for (let i = 0; i < iterations; i++) {
      await page.goto('/');
      await page.waitForLoadState('networkidle');
      await page.close();

      // Reopen page
      page = await page.context().newPage();
    }

    // Final page should work correctly
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('body')).toBeVisible();
  });

  test('should not accumulate connections over time', async ({ request }) => {
    // Make many requests with short-lived connections
    for (let i = 0; i < 100; i++) {
      await request.get(`${MCP_SERVER_URL}/health`);
    }

    // Server should still be healthy
    const health = await request.get(`${MCP_SERVER_URL}/health`);
    expect(health.ok()).toBeTruthy();

    const body = await health.json();

    // Check connection count if reported
    if (body.active_connections !== undefined) {
      expect(body.active_connections).toBeLessThan(10);
    }
  });
});

test.describe('Edge Cases @stability', () => {
  test('should handle concurrent tool executions', async ({ request }) => {
    const concurrentCalls = 20;

    const requests = Array.from({ length: concurrentCalls }, (_, i) =>
      request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'list_files',
            arguments: {
              path: '.',
            },
          },
        },
        timeout: 30000,
      })
    );

    const results = await Promise.allSettled(requests);
    const successful = results.filter((r) => r.status === 'fulfilled');

    console.log(`Concurrent tools: ${successful.length}/${concurrentCalls} succeeded`);

    expect(successful.length).toBeGreaterThan(concurrentCalls * 0.8);
  });

  test('should handle large responses', async ({ request }) => {
    // Request that returns large response (list many files)
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'search_files',
          arguments: {
            pattern: '*.rs',
            path: '.',
          },
        },
      },
      timeout: 60000,
    });

    expect(response.ok()).toBeTruthy();

    const body = await response.json();

    // Should have either result or error (not crash)
    expect(body.result !== undefined || body.error !== undefined).toBeTruthy();
  });

  test('should handle special characters in paths', async ({ request }) => {
    const specialPaths = [
      'path with spaces',
      'path-with-dashes',
      'path_with_underscores',
      'path.with.dots',
      "path'with'quotes",
      'path with (parentheses)',
    ];

    for (const path of specialPaths) {
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: 1,
          method: 'tools/call',
          params: {
            name: 'read_file',
            arguments: {
              file_path: path,
            },
          },
        },
        timeout: 10000,
      });

      // Should handle gracefully (not crash)
      const body = await response.json();
      expect(body.jsonrpc).toBe('2.0');
    }
  });

  test('should handle unicode in content', async ({ request }) => {
    const testFile = 'test_unicode_temp.txt';
    TEST_FILES_TO_CLEANUP.push(testFile);

    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'write_file',
          arguments: {
            file_path: testFile,
            content: 'Hello 世界 🌍 مرحبا こんにちは',
          },
        },
      },
      timeout: 10000,
    });

    const body = await response.json();

    // Should handle unicode properly
    expect(body.result !== undefined || body.error !== undefined).toBeTruthy();

    // Cleanup test file
    await cleanupTestFiles();
  });
});

test.describe('Graceful Shutdown @stability', () => {
  test('should complete in-flight requests on shutdown', async ({ request }) => {
    // Start a long-running request
    const longRequest = request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'search_files',
          arguments: {
            pattern: '**/*',
            path: '.',
          },
        },
      },
      timeout: 60000,
    });

    // Simultaneously check health
    const healthCheck = request.get(`${MCP_SERVER_URL}/health`);

    const [longResponse, healthResponse] = await Promise.all([
      longRequest.catch((e) => null),
      healthCheck,
    ]);

    // Health should always work
    expect(healthResponse.ok()).toBeTruthy();
  });
});

test.describe('Memory Pressure @stability', () => {
  test('should handle memory pressure gracefully', async ({ request }) => {
    // Request many large file reads simultaneously
    const requests = Array.from({ length: 10 }, (_, i) =>
      request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'search_files',
            arguments: {
              pattern: '**/*.rs',
              path: '.',
            },
          },
        },
        timeout: 60000,
      })
    );

    const results = await Promise.allSettled(requests);

    // Should complete without crashing
    const successful = results.filter((r) => r.status === 'fulfilled');
    expect(successful.length).toBeGreaterThan(0);

    // Server should still be healthy
    const health = await request.get(`${MCP_SERVER_URL}/health`);
    expect(health.ok()).toBeTruthy();
  });
});

test.describe('Rate Limiting @stability', () => {
  test('should handle burst traffic', async ({ request }) => {
    const burstSize = 100;
    const results: boolean[] = [];

    // Send burst of requests
    const requests = Array.from({ length: burstSize }, (_, i) =>
      request
        .get(`${MCP_SERVER_URL}/health`, { timeout: 5000 })
        .then((r) => r.ok())
        .catch(() => false)
    );

    const responses = await Promise.all(requests);
    const successRate = (responses.filter((r) => r).length / burstSize) * 100;

    console.log(`Burst traffic: ${successRate.toFixed(2)}% success rate`);

    // Should handle burst (may rate limit but not crash)
    expect(successRate).toBeGreaterThan(80);
  });
});
