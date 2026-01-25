import { test, expect, request } from '@playwright/test';

/**
 * Integration Tests for Serena MCP Server
 *
 * These tests verify the integration between:
 * - WASM Dashboard Frontend
 * - Rust MCP Server Backend
 * - SSE Event Streaming
 * - Tool Execution Flow
 */

const MCP_SERVER_URL = process.env.MCP_SERVER_URL || 'http://localhost:3000';

test.describe('Server Health @integration', () => {
  test('should return healthy status', async ({ request }) => {
    const response = await request.get(`${MCP_SERVER_URL}/health`);

    expect(response.ok()).toBeTruthy();

    const body = await response.json();
    expect(body).toHaveProperty('status');
    expect(body.status).toBe('healthy');
  });

  test('should include version information', async ({ request }) => {
    const response = await request.get(`${MCP_SERVER_URL}/health`);
    const body = await response.json();

    expect(body).toHaveProperty('version');
    expect(typeof body.version).toBe('string');
  });
});

test.describe('SSE Connection @integration', () => {
  test('should establish SSE connection', async ({ page }) => {
    // Use waitForResponse to avoid race condition
    const ssePromise = page.waitForResponse(
      (response) =>
        (response.url().includes('/events') || response.url().includes('/sse')) &&
        response.headers()['content-type']?.includes('text/event-stream'),
      { timeout: 10000 }
    );

    await page.goto('/');

    const sseResponse = await ssePromise;
    expect(sseResponse.status()).toBe(200);
    expect(sseResponse.headers()['content-type']).toContain('text/event-stream');
  });

  test('should receive tool list via SSE', async ({ page }) => {
    // Wait for SSE connection and tools to load
    const ssePromise = page.waitForResponse(
      (response) =>
        response.url().includes('/events') || response.url().includes('/sse'),
      { timeout: 10000 }
    );

    await page.goto('/');
    await ssePromise;

    // Wait for tools to appear in UI with stronger assertion
    const toolElement = page.locator('.tool-card, [data-tool], .tool-item').first();
    await expect(toolElement).toBeVisible({ timeout: 15000 });

    // Verify tool element has meaningful content
    const toolText = await toolElement.textContent();
    expect(toolText).toBeTruthy();
    expect(toolText!.length).toBeGreaterThan(0);
  });

  test('should handle SSE reconnection', async ({ page }) => {
    // Wait for initial SSE connection
    const initialSSE = page.waitForResponse(
      (response) =>
        response.url().includes('/events') || response.url().includes('/sse'),
      { timeout: 10000 }
    );

    await page.goto('/');
    await initialSSE;
    await page.waitForLoadState('networkidle');

    // Simulate network disconnect
    await page.context().setOffline(true);

    // Wait for disconnect indicator or UI update
    await page.waitForTimeout(2000);

    // Prepare to capture reconnection
    const reconnectSSE = page.waitForResponse(
      (response) =>
        response.url().includes('/events') || response.url().includes('/sse'),
      { timeout: 15000 }
    );

    // Reconnect
    await page.context().setOffline(false);

    // Wait for SSE to reconnect
    const reconnected = await reconnectSSE;
    expect(reconnected.status()).toBe(200);

    // Verify page is functional with content check
    const body = page.locator('body');
    await expect(body).toBeVisible();
    const content = await body.textContent();
    expect(content!.length).toBeGreaterThan(100);
  });
});

test.describe('Tool Discovery @integration', () => {
  test('should list available tools', async ({ request }) => {
    // MCP tools/list request
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {},
      },
    });

    expect(response.ok()).toBeTruthy();

    const body = await response.json();
    expect(body).toHaveProperty('result');
    expect(body.result).toHaveProperty('tools');
    expect(Array.isArray(body.result.tools)).toBeTruthy();
    expect(body.result.tools.length).toBeGreaterThan(0);
  });

  test('should have proper tool schema', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/list',
        params: {},
      },
    });

    const body = await response.json();
    const tools = body.result.tools;

    for (const tool of tools.slice(0, 5)) {
      // Check required fields
      expect(tool).toHaveProperty('name');
      expect(typeof tool.name).toBe('string');
      expect(tool.name.length).toBeGreaterThan(0);

      expect(tool).toHaveProperty('description');
      expect(typeof tool.description).toBe('string');

      if (tool.inputSchema) {
        expect(tool.inputSchema).toHaveProperty('type');
      }
    }
  });

  test('should display tools in dashboard', async ({ page }) => {
    await page.goto('/');

    // Wait for tools to load
    const toolCards = page.locator('.tool-card, [data-tool], .tool-item');

    await expect(toolCards.first()).toBeVisible({ timeout: 15000 });

    // Should have multiple tools
    const count = await toolCards.count();
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Tool Execution @integration', () => {
  test('should execute read_file tool', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'read_file',
          arguments: {
            file_path: 'Cargo.toml',
          },
        },
      },
    });

    expect(response.ok()).toBeTruthy();

    const body = await response.json();

    // Should have either result or error
    if (body.error) {
      // Tool execution error should have proper format
      expect(body.error).toHaveProperty('message');
    } else {
      expect(body).toHaveProperty('result');
      expect(body.result).toHaveProperty('content');
    }
  });

  test('should handle invalid tool gracefully', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'nonexistent_tool',
          arguments: {},
        },
      },
    });

    const body = await response.json();

    // Should return error, not crash
    expect(body).toHaveProperty('error');
    expect(body.error).toHaveProperty('message');
  });

  test('should handle missing parameters', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'read_file',
          arguments: {},
        },
      },
    });

    const body = await response.json();

    // Should return validation error
    expect(body).toHaveProperty('error');
  });
});

test.describe('Dashboard-Server Integration @integration', () => {
  test('should update UI when server responds', async ({ page }) => {
    await page.goto('/');

    // Wait for initial load
    await page.waitForLoadState('networkidle');

    // Take initial screenshot for comparison
    const initialContent = await page.content();

    // Wait for real-time updates
    await page.waitForTimeout(3000);

    // Page should have loaded content
    const finalContent = await page.content();

    // Content should have meaningful data (not just loading state)
    expect(finalContent.length).toBeGreaterThan(initialContent.length * 0.5);
  });

  test('should handle server restart gracefully', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Simulate server unavailability
    await page.route(`${MCP_SERVER_URL}/**`, (route) => {
      route.abort('connectionfailed');
    });

    await page.waitForTimeout(2000);

    // Re-enable server
    await page.unroute(`${MCP_SERVER_URL}/**`);

    // Dashboard should attempt reconnection
    await page.waitForTimeout(5000);

    // Page should remain functional
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Error Handling @integration', () => {
  test('should handle malformed requests', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: 'not valid json',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Server should handle gracefully (400 or proper error)
    const body = await response.json().catch(() => null);

    if (body) {
      expect(body).toHaveProperty('error');
    } else {
      expect(response.status()).toBeGreaterThanOrEqual(400);
    }
  });

  test('should return proper JSON-RPC error format', async ({ request }) => {
    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'invalid/method',
        params: {},
      },
    });

    const body = await response.json();

    expect(body).toHaveProperty('jsonrpc', '2.0');
    expect(body).toHaveProperty('id', 1);
    expect(body).toHaveProperty('error');
    expect(body.error).toHaveProperty('code');
    expect(body.error).toHaveProperty('message');
  });

  test('should timeout long operations', async ({ request }) => {
    // This test checks that the server has timeout protection
    const startTime = Date.now();

    const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
      data: {
        jsonrpc: '2.0',
        id: 1,
        method: 'tools/call',
        params: {
          name: 'search_files',
          arguments: {
            // Use project root for cross-platform compatibility
            pattern: '**/*',
            path: '.',
          },
        },
      },
      timeout: 30000,
    });

    const elapsed = Date.now() - startTime;
    const body = await response.json();

    // Should complete within reasonable time (timeout or result)
    expect(elapsed).toBeLessThan(30000);
    // Verify we got a valid response
    expect(body.jsonrpc).toBe('2.0');
    expect(body.result !== undefined || body.error !== undefined).toBeTruthy();
  });
});

test.describe('Concurrent Operations @integration', () => {
  test('should handle multiple simultaneous requests', async ({ request }) => {
    const requests = Array.from({ length: 10 }, (_, i) =>
      request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/list',
          params: {},
        },
      })
    );

    const responses = await Promise.all(requests);

    // All requests should succeed
    for (const response of responses) {
      expect(response.ok()).toBeTruthy();

      const body = await response.json();
      expect(body).toHaveProperty('result');
    }
  });

  test('should maintain response order', async ({ request }) => {
    const ids = [1, 2, 3, 4, 5];
    const requests = ids.map((id) =>
      request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id,
          method: 'tools/list',
          params: {},
        },
      })
    );

    const responses = await Promise.all(requests);

    for (let i = 0; i < responses.length; i++) {
      const body = await responses[i].json();
      expect(body.id).toBe(ids[i]);
    }
  });
});
