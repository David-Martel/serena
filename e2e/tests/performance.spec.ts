import { test, expect } from '@playwright/test';

/**
 * Performance Tests for Serena MCP Server
 *
 * These tests measure:
 * - Response latency
 * - Throughput under load
 * - Resource utilization
 * - Startup time
 */

const MCP_SERVER_URL = process.env.MCP_SERVER_URL || 'http://localhost:3000';

/**
 * Calculate percentile using linear interpolation
 * @param sorted - Pre-sorted array of numbers
 * @param percentile - Percentile to calculate (0-100)
 */
function calculatePercentile(sorted: number[], percentile: number): number {
  if (sorted.length === 0) return 0;
  if (sorted.length === 1) return sorted[0];

  // Linear interpolation for accurate percentiles
  const index = (percentile / 100) * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  const weight = index % 1;

  if (lower === upper) return sorted[lower];
  return sorted[lower] * (1 - weight) + sorted[upper] * weight;
}

// Performance thresholds (in milliseconds)
const THRESHOLDS = {
  healthCheck: 100,
  toolsList: 200,
  simpleToolCall: 500,
  pageLoad: 3000,
  wasmInit: 5000,
};

test.describe('Response Latency @performance', () => {
  test('health check should respond within threshold', async ({ request }) => {
    const times: number[] = [];

    // Run multiple iterations for accurate measurement
    for (let i = 0; i < 10; i++) {
      const start = performance.now();
      const response = await request.get(`${MCP_SERVER_URL}/health`);
      const end = performance.now();

      expect(response.ok()).toBeTruthy();
      times.push(end - start);
    }

    // Calculate statistics
    const avg = times.reduce((a, b) => a + b, 0) / times.length;
    const sortedTimes = [...times].sort((a, b) => a - b);
    const p95 = calculatePercentile(sortedTimes, 95);

    console.log(`Health check latency - Avg: ${avg.toFixed(2)}ms, P95: ${p95.toFixed(2)}ms`);

    expect(avg).toBeLessThan(THRESHOLDS.healthCheck);
  });

  test('tools/list should respond within threshold', async ({ request }) => {
    const times: number[] = [];

    for (let i = 0; i < 10; i++) {
      const start = performance.now();
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/list',
          params: {},
        },
      });
      const end = performance.now();

      expect(response.ok()).toBeTruthy();
      times.push(end - start);
    }

    const avg = times.reduce((a, b) => a + b, 0) / times.length;
    const sortedTimes = [...times].sort((a, b) => a - b);
    const p95 = calculatePercentile(sortedTimes, 95);

    console.log(`tools/list latency - Avg: ${avg.toFixed(2)}ms, P95: ${p95.toFixed(2)}ms`);

    expect(avg).toBeLessThan(THRESHOLDS.toolsList);
  });

  test('simple tool call should respond within threshold', async ({ request }) => {
    const times: number[] = [];

    for (let i = 0; i < 5; i++) {
      const start = performance.now();
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/call',
          params: {
            name: 'read_file',
            arguments: {
              file_path: 'Cargo.toml',
            },
          },
        },
      });
      const end = performance.now();

      expect(response.ok()).toBeTruthy();
      times.push(end - start);
    }

    const avg = times.reduce((a, b) => a + b, 0) / times.length;
    const sortedTimes = [...times].sort((a, b) => a - b);
    const p95 = calculatePercentile(sortedTimes, 95);

    console.log(`Tool call latency - Avg: ${avg.toFixed(2)}ms, P95: ${p95.toFixed(2)}ms`);

    expect(avg).toBeLessThan(THRESHOLDS.simpleToolCall);
  });
});

test.describe('Throughput @performance', () => {
  test('should handle high request rate', async ({ request }) => {
    const startTime = performance.now();
    const requests = 50;
    const concurrent = 10;

    let completed = 0;
    let failed = 0;

    const batches = Math.ceil(requests / concurrent);

    for (let batch = 0; batch < batches; batch++) {
      const batchRequests = Array.from(
        { length: Math.min(concurrent, requests - batch * concurrent) },
        (_, i) =>
          request
            .post(`${MCP_SERVER_URL}/rpc`, {
              data: {
                jsonrpc: '2.0',
                id: batch * concurrent + i + 1,
                method: 'tools/list',
                params: {},
              },
            })
            .then((r) => {
              if (r.ok()) completed++;
              else failed++;
            })
            .catch(() => {
              failed++;
            })
      );

      await Promise.all(batchRequests);
    }

    const endTime = performance.now();
    const duration = (endTime - startTime) / 1000;
    const throughput = completed / duration;

    console.log(`Throughput: ${throughput.toFixed(2)} req/s (${completed}/${requests} succeeded in ${duration.toFixed(2)}s)`);

    expect(completed).toBeGreaterThan(requests * 0.95); // 95% success rate
    expect(throughput).toBeGreaterThan(10); // At least 10 req/s
  });

  test('should maintain latency under load', async ({ request }) => {
    const concurrentRequests = 20;
    const times: number[] = [];

    const requests = Array.from({ length: concurrentRequests }, async (_, i) => {
      const start = performance.now();
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/list',
          params: {},
        },
      });
      const end = performance.now();

      if (response.ok()) {
        times.push(end - start);
      }
    });

    await Promise.all(requests);

    if (times.length === 0) {
      throw new Error('All requests failed under load');
    }

    const avg = times.reduce((a, b) => a + b, 0) / times.length;
    const max = Math.max(...times);
    const sortedTimes = [...times].sort((a, b) => a - b);
    const p99 = calculatePercentile(sortedTimes, 99);

    console.log(`Under load - Avg: ${avg.toFixed(2)}ms, P99: ${p99.toFixed(2)}ms, Max: ${max.toFixed(2)}ms`);

    // Latency should not degrade too much under load
    expect(avg).toBeLessThan(THRESHOLDS.toolsList * 3);
    expect(p99).toBeLessThan(THRESHOLDS.toolsList * 5);
  });
});

test.describe('Dashboard Performance @performance', () => {
  test('page should load within threshold', async ({ page }) => {
    const start = performance.now();

    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    const domLoaded = performance.now() - start;

    await page.waitForLoadState('networkidle');

    const networkIdle = performance.now() - start;

    console.log(`Page load - DOM: ${domLoaded.toFixed(2)}ms, Network idle: ${networkIdle.toFixed(2)}ms`);

    expect(domLoaded).toBeLessThan(THRESHOLDS.pageLoad);
  });

  test('WASM should initialize within threshold', async ({ page }) => {
    const start = performance.now();

    await page.goto('/');

    // Wait for WASM to initialize (content to appear)
    await page.waitForFunction(
      () => {
        const body = document.body;
        return body.textContent && body.textContent.length > 100;
      },
      { timeout: THRESHOLDS.wasmInit }
    );

    const elapsed = performance.now() - start;

    console.log(`WASM init time: ${elapsed.toFixed(2)}ms`);

    expect(elapsed).toBeLessThan(THRESHOLDS.wasmInit);
  });

  test('should measure First Contentful Paint', async ({ page }) => {
    await page.goto('/');

    // Get performance metrics
    const metrics = await page.evaluate(() => {
      const entries = performance.getEntriesByType('paint');
      const fcp = entries.find((e) => e.name === 'first-contentful-paint');
      const fp = entries.find((e) => e.name === 'first-paint');

      return {
        firstPaint: fp?.startTime,
        firstContentfulPaint: fcp?.startTime,
      };
    });

    console.log(`FCP: ${metrics.firstContentfulPaint?.toFixed(2)}ms, FP: ${metrics.firstPaint?.toFixed(2)}ms`);

    if (metrics.firstContentfulPaint) {
      expect(metrics.firstContentfulPaint).toBeLessThan(3000);
    }
  });

  test('should measure Largest Contentful Paint', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Wait a bit for LCP to be calculated
    await page.waitForTimeout(2000);

    const lcp = await page.evaluate(() => {
      return new Promise<number | null>((resolve) => {
        new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const lastEntry = entries[entries.length - 1];
          resolve(lastEntry?.startTime);
        }).observe({ type: 'largest-contentful-paint', buffered: true });

        // Fallback if no LCP
        setTimeout(() => resolve(null), 1000);
      });
    });

    if (lcp) {
      console.log(`LCP: ${lcp.toFixed(2)}ms`);
      expect(lcp).toBeLessThan(4000);
    }
  });
});

test.describe('Memory Usage @performance', () => {
  test('should not leak memory on repeated requests', async ({ request }) => {
    // Baseline memory check
    const initialHealth = await request.get(`${MCP_SERVER_URL}/health`);
    const initialBody = await initialHealth.json();
    const initialMemory = initialBody.memory_mb;

    // Skip test with clear message if memory metrics unavailable
    test.skip(
      initialMemory === undefined,
      'Server does not expose memory_mb in health endpoint - memory leak test skipped'
    );

    // Make many requests
    for (let i = 0; i < 100; i++) {
      await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/list',
          params: {},
        },
      });
    }

    // Check memory after requests
    const finalHealth = await request.get(`${MCP_SERVER_URL}/health`);
    const finalBody = await finalHealth.json();
    const finalMemory = finalBody.memory_mb;

    expect(finalMemory).toBeDefined();

    const memoryGrowth = finalMemory - initialMemory;
    console.log(`Memory: ${initialMemory}MB -> ${finalMemory}MB (growth: ${memoryGrowth}MB)`);

    // Memory should not grow excessively
    expect(memoryGrowth).toBeLessThan(50); // Less than 50MB growth
  });

  test('should not leak memory in browser', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Get initial heap size
    const initialHeap = await page.evaluate(() => {
      // @ts-ignore - performance.memory is Chrome-specific
      const memory = performance.memory;
      return memory ? memory.usedJSHeapSize / 1024 / 1024 : null;
    });

    // Skip test with clear message if memory API unavailable (non-Chromium browsers)
    test.skip(
      initialHeap === null,
      'Browser does not support performance.memory API (Chromium-only) - browser memory test skipped'
    );

    // Interact with the page
    for (let i = 0; i < 20; i++) {
      await page.reload();
      await page.waitForLoadState('networkidle');
    }

    // Get final heap size
    const finalHeap = await page.evaluate(() => {
      // @ts-ignore - performance.memory is Chrome-specific
      const memory = performance.memory;
      return memory ? memory.usedJSHeapSize / 1024 / 1024 : null;
    });

    expect(finalHeap).not.toBeNull();

    const heapGrowth = finalHeap! - initialHeap!;
    console.log(`Browser heap: ${initialHeap!.toFixed(2)}MB -> ${finalHeap!.toFixed(2)}MB (growth: ${heapGrowth.toFixed(2)}MB)`);

    // Heap should not grow excessively
    expect(heapGrowth).toBeLessThan(50);
  });
});

test.describe('Bundle Size @performance', () => {
  test('should have reasonable WASM bundle size', async ({ page }) => {
    const resourceSizes: { name: string; size: number }[] = [];

    page.on('response', async (response) => {
      const url = response.url();
      const headers = response.headers();
      const contentLength = parseInt(headers['content-length'] || '0', 10);

      if (url.includes('.wasm') || url.includes('.js') || url.includes('.css')) {
        resourceSizes.push({
          name: url.split('/').pop() || url,
          size: contentLength,
        });
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const wasmBundles = resourceSizes.filter((r) => r.name.includes('.wasm'));
    const jsBundles = resourceSizes.filter((r) => r.name.includes('.js'));
    const cssBundles = resourceSizes.filter((r) => r.name.includes('.css'));

    const totalWasm = wasmBundles.reduce((sum, r) => sum + r.size, 0) / 1024 / 1024;
    const totalJs = jsBundles.reduce((sum, r) => sum + r.size, 0) / 1024;
    const totalCss = cssBundles.reduce((sum, r) => sum + r.size, 0) / 1024;

    console.log(`Bundle sizes - WASM: ${totalWasm.toFixed(2)}MB, JS: ${totalJs.toFixed(2)}KB, CSS: ${totalCss.toFixed(2)}KB`);

    // WASM should be reasonable size (< 5MB)
    if (totalWasm > 0) {
      expect(totalWasm).toBeLessThan(5);
    }
  });
});
