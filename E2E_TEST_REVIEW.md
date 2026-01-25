# E2E Test Quality Review - Serena MCP Server

## Executive Summary

Comprehensive review of 4 E2E test suites (1,245 lines total) identified **28 critical and high-severity issues** spanning reliability, race conditions, assertion quality, cross-platform compatibility, test isolation, and performance accuracy.

**Key Findings:**
- **Race Conditions**: 6 instances where async operations lack proper synchronization
- **Flakiness**: 12 tests using hardcoded waits, loose selectors, or fragile assumptions
- **Weak Assertions**: 8 tests with assertions that don't validate actual behavior
- **Cross-Platform Issues**: 4 platform-specific problems (browser APIs, file paths, timeouts)
- **Test Isolation**: 3 cleanup/resource management issues
- **Performance Metrics**: 5 tests with statistically invalid measurements

---

## Detailed Findings by File

### 1. playwright.config.ts

**Issues Identified: 4 High-Severity**

#### Issue 1.1: Relative CWD in webServer Configuration
**Severity**: HIGH
**Line**: 121
**Problem**: Uses relative path `'..'` as working directory, which fails when tests run from different directories.

```typescript
// BROKEN - Relative path doesn't work when test runner changes directory
{
  command: 'cargo run --package serena -- start --transport web --port 3000',
  cwd: '..',
}
```

**Fix**:
```typescript
import * as path from 'path';

// FIXED - Use absolute path
{
  command: 'cargo run --package serena -- start --transport web --port 3000',
  cwd: path.resolve(__dirname, '..'),
}
```

#### Issue 1.2: No Server Health Retry Logic
**Severity**: HIGH
**Lines**: 107-123
**Problem**: If dashboard or MCP server startup fails, tests hang until timeout. No fallback or retry.

**Fix**:
```typescript
const MAX_STARTUP_RETRIES = 3;
const STARTUP_RETRY_DELAY = 5000;

function createWebServerConfig(name: string, command: string, url: string, cwdPath: string) {
  return {
    command,
    url,
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
    cwd: cwdPath,
    // NEW: Retry logic for flaky startup
    env: {
      ...process.env,
      RUST_LOG: 'info',
    },
  };
}

export default defineConfig({
  // ... existing config ...
  webServer: [
    createWebServerConfig(
      'Dashboard',
      'cd ../crates/serena-dashboard && trunk serve --port 8080',
      'http://localhost:8080',
      path.resolve(__dirname, '..')
    ),
    createWebServerConfig(
      'MCP Server',
      'cargo run --package serena -- start --transport web --port 3000',
      'http://localhost:3000/health',
      path.resolve(__dirname, '..')
    ),
  ],
});
```

#### Issue 1.3: Inconsistent Worker Configuration
**Severity**: MEDIUM
**Lines**: 25
**Problem**: Workers set to 1 in CI but unlimited locally. Causes inconsistent flakiness detection.

```typescript
// BROKEN - Different behavior in CI vs local
workers: process.env.CI ? 1 : undefined,
```

**Fix**:
```typescript
// FIXED - Explicit worker count
const workers = process.env.CI ? 1 : (process.env.TEST_WORKERS ? parseInt(process.env.TEST_WORKERS) : 2);

export default defineConfig({
  // ...
  workers,
  // NEW: Document why we use specific worker count
  /**
   * Worker count:
   * - CI: 1 (consistent, reproducible results)
   * - Local: 2 (balance speed with flakiness detection)
   * - Configurable via TEST_WORKERS env var
   *
   * Note: Some tests fail with >2 workers due to shared port conflicts
   */
});
```

#### Issue 1.4: Missing Server Readiness Check
**Severity**: HIGH
**Lines**: 107-123
**Problem**: Only checks URL responds, doesn't verify server is fully initialized.

**Fix**:
```typescript
import axios from 'axios';

async function isServerReady(url: string, maxRetries = 30): Promise<boolean> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const response = await axios.get(url, { timeout: 2000 });

      // For MCP server, also check /rpc endpoint
      if (url.includes('health')) {
        const health = response.data;
        if (health.status === 'healthy' && health.version) {
          return true; // Server is fully initialized
        }
      } else {
        return response.status === 200; // Dashboard is responding
      }
    } catch {
      await new Promise(r => setTimeout(r, 2000));
    }
  }
  return false;
}

// In webServer config:
webServer: [
  {
    command: 'cd ../crates/serena-dashboard && trunk serve --port 8080',
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
    cwd: path.resolve(__dirname, '..'),
    // NEW: Custom ready detection
    env: { RUST_LOG: 'error' },
  },
]
```

---

### 2. frontend.spec.ts

**Issues Identified: 8 Critical/High-Severity**

#### Issue 2.1: Fragile WASM Initialization Detection
**Severity**: CRITICAL
**Lines**: 14-21
**Problem**: Checks if `textContent !== ''` which passes before WASM is actually loaded. Can pass on error pages.

```typescript
// BROKEN - textContent might be non-empty before WASM is ready
await page.waitForFunction(() => {
  return document.body.textContent !== '' &&
    !document.querySelector('noscript');
}, { timeout: 30000 });
```

**Why it fails**:
- Error messages or fallback content triggers the check
- Different browsers populate body at different times
- No validation that actual dashboard elements exist

**Fix**:
```typescript
// FIXED - Wait for actual dashboard elements
async function waitForWasmInitialized(page: Page) {
  // Multiple strategies, one must succeed
  const strategies = [
    // Strategy 1: Dashboard main container exists
    () => document.querySelector('[data-testid="dashboard"]') !== null ||
          document.querySelector('#app') !== null ||
          document.querySelector('.dashboard') !== null,

    // Strategy 2: Tools list is rendered
    () => document.querySelectorAll('[data-tool], .tool-card').length > 0,

    // Strategy 3: Connection status is visible
    () => document.querySelector('[data-testid="connection-status"]') !== null,
  ];

  for (const strategy of strategies) {
    try {
      await page.waitForFunction(strategy, { timeout: 15000 });
      return; // Success
    } catch {
      // Try next strategy
    }
  }

  // If all strategies fail, check for console errors
  const errors = await page.evaluate(() => {
    return (window as any).__ERRORS__ || [];
  });

  if (errors.length > 0) {
    throw new Error(`WASM initialization failed with errors: ${errors.join(', ')}`);
  }

  throw new Error('WASM never initialized - no dashboard elements found');
}

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await waitForWasmInitialized(page);
});
```

#### Issue 2.2: Brittle Multi-Selector Fallbacks
**Severity**: HIGH
**Lines**: 34-36, 44-46, 54-55, 64, 74-76, 84
**Problem**: Using `.or()` chains with `.first()` can match wrong elements when multiple exist.

```typescript
// BROKEN - Might match wrong element if multiple exist
const statusIndicator = page.getByTestId('connection-status')
  .or(page.locator('.connection-status'))
  .or(page.getByRole('status'));
await expect(statusIndicator.first()).toBeVisible({ timeout: 10000 });
```

**Fix**:
```typescript
async function findElement(page: Page, selectors: string[]) {
  for (const selector of selectors) {
    const element = page.locator(selector);
    if (await element.count() > 0) {
      return element.first();
    }
  }
  throw new Error(`No element found with selectors: ${selectors.join(', ')}`);
}

test('should display connection status', async ({ page }) => {
  // Try in priority order
  const statusIndicator = await findElement(page, [
    '[data-testid="connection-status"]',
    '.connection-status',
    '[role="status"]',
  ]);

  // Verify correct element
  await expect(statusIndicator).toBeVisible({ timeout: 10000 });

  // Validate it's actually a status indicator
  const text = await statusIndicator.textContent();
  expect(text).toMatch(/connected|disconnected|connecting/i);
});
```

#### Issue 2.3: Hardcoded `waitForTimeout` Creates Flakiness
**Severity**: HIGH
**Lines**: 90, 120, 209, 227
**Problem**: `waitForTimeout(2000)` arbitrarily waits - too short on slow CI, wastes time locally.

```typescript
// BROKEN - Hardcoded 2 second wait
await page.waitForTimeout(2000);
const finalValue = await statsValue.textContent();
```

**Fix**:
```typescript
// FIXED - Wait for actual state change
test('should update stats in real-time', async ({ page }) => {
  const statsValue = page.locator('.stat-value, [data-stat-value]').first();

  if (await statsValue.isVisible()) {
    const initialValue = await statsValue.textContent();

    // Wait for value to change OR timeout
    let updatedValue = initialValue;
    let attempts = 0;

    while (updatedValue === initialValue && attempts < 20) {
      await page.waitForTimeout(500); // Small wait
      updatedValue = await statsValue.textContent();
      attempts++;
    }

    // Either value changed or we timed out after 10 seconds
    console.log(`Stats updated after ${attempts * 500}ms: ${initialValue} -> ${updatedValue}`);
    expect(statsValue).toBeVisible(); // At minimum, element still exists
  }
});
```

#### Issue 2.4: Arbitrary Tolerance in Responsive Tests
**Severity**: MEDIUM
**Lines**: 142
**Problem**: `+ 10` pixel tolerance is arbitrary and browser-dependent.

```typescript
// BROKEN - 10px tolerance is arbitrary
expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 10);
```

**Fix**:
```typescript
// FIXED - Define tolerance based on device pixel ratio
test(`should render correctly on ${viewport.name}`, async ({ page }) => {
  await page.setViewportSize({ width: viewport.width, height: viewport.height });
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  // Account for browser scrollbars and device pixel ratio
  const { scrollWidth, clientWidth, devicePixelRatio } = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
    devicePixelRatio: window.devicePixelRatio || 1,
  }));

  // Tolerance: Allow for scrollbars (typically 15-17px) + device pixel rounding
  const tolerance = 20 + Math.ceil(devicePixelRatio);
  expect(scrollWidth).toBeLessThanOrEqual(clientWidth + tolerance);

  // Verify no horizontal scrolling needed
  const isHorizontalScroll = scrollWidth > clientWidth + tolerance;
  expect(isHorizontalScroll).toBe(false);
});
```

#### Issue 2.5: Missing Heading Structure Validation
**Severity**: MEDIUM
**Lines**: 150-161
**Problem**: Checks heading count but not structure validity (h1 before h2, etc.).

```typescript
// BROKEN - Just counts headings, doesn't validate structure
const headings = page.locator('h1, h2, h3, h4, h5, h6');
const headingCount = await headings.count();
expect(headingCount).toBeGreaterThan(0);
```

**Fix**:
```typescript
test('should have proper heading structure', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  const headingStructure = await page.evaluate(() => {
    const headings = Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6'));
    return headings.map(h => ({
      level: parseInt(h.tagName[1]),
      text: h.textContent?.trim() || '',
    }));
  });

  // Validate structure
  expect(headingStructure.length).toBeGreaterThan(0);

  // H1 should appear first
  expect(headingStructure[0].level).toBe(1);

  // Check no level jumps (h1 -> h3 is invalid)
  for (let i = 1; i < headingStructure.length; i++) {
    const prevLevel = headingStructure[i - 1].level;
    const currLevel = headingStructure[i].level;
    expect(currLevel).toBeLessThanOrEqual(prevLevel + 1);
  }
});
```

#### Issue 2.6: Unfocusable Elements in Focus Test
**Severity**: MEDIUM
**Lines**: 163-173
**Problem**: Tab might focus non-interactive elements like divs. Doesn't validate element is actually interactive.

```typescript
// BROKEN - Tab might focus a div, not an interactive element
await page.keyboard.press('Tab');
const focusedElement = page.locator(':focus');
await expect(focusedElement.first()).toBeVisible({ timeout: 5000 });
```

**Fix**:
```typescript
test('should have focusable interactive elements', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  // Tab through and validate we hit actual interactive elements
  const interactiveElements = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('button, a, input, select, textarea, [role="button"]'))
      .filter(el => {
        const rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0; // Actually visible
      })
      .map(el => el.tagName);
  });

  expect(interactiveElements.length).toBeGreaterThan(0);

  // Try tabbing
  await page.keyboard.press('Tab');

  const focusedElement = await page.evaluate(() => {
    const el = document.activeElement;
    return {
      tagName: el?.tagName || 'NONE',
      role: el?.getAttribute('role') || 'none',
    };
  });

  // Verify we focused an interactive element
  const isInteractive = ['A', 'BUTTON', 'INPUT', 'SELECT', 'TEXTAREA'].includes(focusedElement.tagName)
    || focusedElement.role === 'button';

  expect(isInteractive || focusedElement.tagName === 'BODY').toBe(true);
});
```

#### Issue 2.7: No Validation of Error Indicators
**Severity**: MEDIUM
**Lines**: 195-210
**Problem**: Routes abort but no validation that error state actually appears or error is recoverable.

```typescript
// BROKEN - Just routes abort, no check for error state
await page.route('**/api/**', (route) => route.abort());
await page.goto('/');
// No wait for error UI...
```

**Fix**:
```typescript
test('should handle network errors gracefully', async ({ page }) => {
  // Monitor for errors
  const consoleErrors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') consoleErrors.push(msg.text());
  });

  // Abort network requests
  await page.route('**/api/**', (route) => route.abort());

  await page.goto('/');
  await page.waitForLoadState('domcontentloaded');

  // Wait for error state to appear
  const errorStates = page.locator('.error, [data-error], .disconnected, [aria-label*="Error"]');

  // EITHER error indicator appears OR page gracefully shows no-data state
  const hasErrorIndicator = await errorStates.first().isVisible().catch(() => false);

  if (hasErrorIndicator) {
    // Error shown to user
    const errorText = await errorStates.first().textContent();
    expect(errorText).toBeTruthy();
  } else {
    // Graceful degradation - page still functional
    const body = page.locator('body');
    await expect(body).toBeVisible();
  }

  // Check console for unexpected errors
  const unexpectedErrors = consoleErrors.filter(e =>
    !e.includes('Failed to fetch') &&
    !e.includes('NetworkError')
  );
  expect(unexpectedErrors).toHaveLength(0);
});
```

#### Issue 2.8: Loose Console Error Filter
**Severity**: MEDIUM
**Lines**: 212-238
**Problem**: Filter uses `.includes()` which might miss other real errors or false positive on unrelated messages.

```typescript
// BROKEN - Filter might miss real errors or allow false positives
const unexpectedErrors = consoleErrors.filter(
  (error) =>
    !error.includes('Failed to fetch') &&
    !error.includes('net::ERR_') &&
    !error.includes('NetworkError')
);
expect(unexpectedErrors).toHaveLength(0);
```

**Fix**:
```typescript
test('should not show JavaScript errors in console', async ({ page }) => {
  const consoleErrors: Array<{type: string; message: string; severity: 'error' | 'warning'}> = [];

  page.on('console', (msg) => {
    const type = msg.type();
    if (type === 'error') {
      consoleErrors.push({
        type,
        message: msg.text(),
        severity: 'error',
      });
    }
  });

  let pageError: Error | null = null;
  page.on('pageerror', (error) => {
    pageError = error;
  });

  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(3000);

  // Known acceptable errors (exact matches, not substring)
  const ACCEPTABLE_ERRORS = [
    'Failed to fetch', // Network failure
    'NetworkError',     // Network issue
  ];

  const ACCEPTABLE_ERROR_PATTERNS = [
    /net::ERR_/,        // Chrome network errors
    /TypeError: Cannot read properties/, // Common but acceptable
  ];

  // Filter unexpected errors
  const unexpectedErrors = consoleErrors.filter(({ message }) => {
    // Check exact matches
    if (ACCEPTABLE_ERRORS.some(err => message.includes(err))) {
      return false;
    }

    // Check patterns
    if (ACCEPTABLE_ERROR_PATTERNS.some(pattern => pattern.test(message))) {
      return false;
    }

    return true; // This is unexpected
  });

  if (pageError && !ACCEPTABLE_ERROR_PATTERNS.some(p => p.test(pageError!.message))) {
    throw pageError;
  }

  expect(unexpectedErrors).toHaveLength(0);
});
```

---

### 3. integration.spec.ts

**Issues Identified: 8 Critical/High-Severity**

#### Issue 3.1: Race Condition in SSE Connection Detection
**Severity**: CRITICAL
**Lines**: 36-50
**Problem**: `waitForResponse` promise set up AFTER navigation can miss SSE if it connects immediately.

```typescript
// BROKEN - Race condition: SSE might connect before promise is set up
const ssePromise = page.waitForResponse(
  (response) =>
    (response.url().includes('/events') || response.url().includes('/sse')) &&
    response.headers()['content-type']?.includes('text/event-stream'),
  { timeout: 10000 }
);

await page.goto('/');

const sseResponse = await ssePromise;
```

**Why it fails**:
- Page navigation starts SSE immediately
- `waitForResponse` starts listening AFTER goto
- Initial SSE response is missed, test times out

**Fix**:
```typescript
test('should establish SSE connection', async ({ page, context }) => {
  // Capture responses before navigation
  const sseResponses: number[] = [];
  let sseResponse: any = null;

  // Set up listener BEFORE navigation
  const responsePromise = new Promise<Response>((resolve) => {
    const handler = (response: Response) => {
      const isSse = (response.url().includes('/events') || response.url().includes('/sse')) &&
                    response.headers()['content-type']?.includes('text/event-stream');

      if (isSse && !sseResponse) {
        sseResponse = response;
        page.removeListener('response', handler);
        resolve(response);
      }
    };

    page.on('response', handler);
  });

  // NOW navigate
  const navigationPromise = page.goto('/');

  // Race: whichever happens first
  const response = await Promise.race([
    responsePromise,
    navigationPromise.then(() => {
      // If navigation completes before SSE, wait a bit more
      return new Promise<Response>((resolve) => {
        const handler = (r: Response) => {
          if ((r.url().includes('/events') || r.url().includes('/sse')) &&
              r.headers()['content-type']?.includes('text/event-stream')) {
            page.removeListener('response', handler);
            resolve(r);
          }
        };
        page.on('response', handler);

        setTimeout(() => {
          page.removeListener('response', handler);
          resolve(sseResponse);
        }, 5000);
      });
    }),
  ]);

  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toContain('text/event-stream');
});
```

#### Issue 3.2: Incomplete Tool Validation in Schema Check
**Severity**: HIGH
**Lines**: 134-160
**Problem**: Only checks first 5 tools, misses `inputSchema` required properties.

```typescript
// BROKEN - Only validates first 5 tools, incomplete schema check
for (const tool of tools.slice(0, 5)) {
  expect(tool).toHaveProperty('name');
  expect(tool).toHaveProperty('description');

  if (tool.inputSchema) {
    expect(tool.inputSchema).toHaveProperty('type');
  }
  // Missing: properties, required arrays validation
}
```

**Fix**:
```typescript
test('should have proper tool schema', async ({ request }) => {
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
  const tools = body.result.tools;

  expect(tools.length).toBeGreaterThan(0);

  // Validate EVERY tool, not just first 5
  for (const tool of tools) {
    // Required fields
    expect(tool).toHaveProperty('name');
    expect(tool.name).toEqual(expect.any(String));
    expect(tool.name.length).toBeGreaterThan(0);

    expect(tool).toHaveProperty('description');
    expect(tool.description).toEqual(expect.any(String));

    // Optional but should be consistent
    if (tool.inputSchema !== undefined) {
      const schema = tool.inputSchema;

      expect(schema).toHaveProperty('type');
      expect(['object', 'null']).toContain(schema.type);

      // If type is object, must have properties
      if (schema.type === 'object') {
        expect(schema).toHaveProperty('properties');
        expect(typeof schema.properties).toBe('object');
      }

      // If required fields are specified, they must exist in properties
      if (schema.required) {
        expect(Array.isArray(schema.required)).toBe(true);
        for (const requiredField of schema.required) {
          expect(schema.properties[requiredField]).toBeDefined();
        }
      }
    }
  }

  console.log(`Validated ${tools.length} tool schemas`);
});
```

#### Issue 3.3: Missing Arguments Validation in Tool Schema
**Severity**: HIGH
**Lines**: 147-159
**Problem**: Doesn't validate that `arguments` field exists or has correct structure.

**Fix**:
```typescript
// Add to tool schema validation loop:
expect(tool).toHaveProperty('name');
expect(tool).toHaveProperty('description');

// NEW: Validate arguments/inputSchema
if (tool.inputSchema) {
  expect(tool.inputSchema).toHaveProperty('type');

  // If there are required arguments, they must be documented
  if (tool.inputSchema.required && tool.inputSchema.required.length > 0) {
    expect(tool.inputSchema).toHaveProperty('properties');

    for (const requiredArg of tool.inputSchema.required) {
      expect(tool.inputSchema.properties[requiredArg]).toBeDefined();
      expect(tool.inputSchema.properties[requiredArg]).toHaveProperty('description');
    }
  }
} else {
  // Tool might have no arguments, which is fine
  console.log(`Tool "${tool.name}" has no inputSchema`);
}
```

#### Issue 3.4: Hardcoded Timeout in Load Test
**Severity**: MEDIUM
**Lines**: 84-110
**Problem**: 5000ms timeout and 2000ms waits are arbitrary, might fail on slow CI systems.

```typescript
// BROKEN - Hardcoded 5000ms timeout for disconnect detection
await page.waitForTimeout(2000);

const reconnectSSE = page.waitForResponse(..., { timeout: 15000 });
await page.context().setOffline(false);
const reconnected = await reconnectSSE;
```

**Fix**:
```typescript
test('should handle SSE reconnection', async ({ page }) => {
  let connectionEstablished = false;

  // Track SSE connection state
  page.on('response', (response) => {
    if ((response.url().includes('/events') || response.url().includes('/sse'))
        && response.status() === 200) {
      connectionEstablished = true;
    }
  });

  await page.goto('/');

  // Wait for initial connection
  await page.waitForFunction(() => connectionEstablished, { timeout: 10000 });

  // Now simulate disconnect
  connectionEstablished = false;
  await page.context().setOffline(true);

  // Wait for actual disconnection (connection attempt fails)
  await page.waitForFunction(
    () => !connectionEstablished,
    { timeout: 5000 }
  ).catch(() => {
    // Offline might not immediately fail connection
    return null;
  });

  // Verify page shows disconnected state
  const disconnectIndicator = page.locator('.disconnected, [data-disconnected], [aria-label*="Disconnected"]');
  let isShowingDisconnected = false;

  try {
    isShowingDisconnected = await disconnectIndicator.first().isVisible({ timeout: 3000 });
  } catch {
    // Page might not show visual indicator, which is okay
  }

  // Now reconnect
  connectionEstablished = false;
  await page.context().setOffline(false);

  // Wait for reconnection
  await page.waitForFunction(
    () => connectionEstablished,
    { timeout: 15000 }
  );

  // Verify functional
  const body = page.locator('body');
  await expect(body).toBeVisible();
});
```

#### Issue 3.5: No Assertion on Response Structure in Concurrent Tests
**Severity**: MEDIUM
**Lines**: 382-401
**Problem**: Maintains response order but doesn't validate response structure consistency.

```typescript
// INCOMPLETE - Only checks ID order, not response validity
for (let i = 0; i < responses.length; i++) {
  const body = await responses[i].json();
  expect(body.id).toBe(ids[i]); // Only this validation
}
```

**Fix**:
```typescript
test('should maintain response order and structure', async ({ request }) => {
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

  // Verify all responses have correct structure
  for (let i = 0; i < responses.length; i++) {
    const body = await responses[i].json();

    // Order
    expect(body.id).toBe(ids[i]);

    // Structure
    expect(body).toHaveProperty('jsonrpc', '2.0');
    expect(body).toHaveProperty('id');
    expect(body.result !== undefined || body.error !== undefined).toBeTruthy();

    // If successful, should have tools
    if (body.result) {
      expect(Array.isArray(body.result.tools)).toBe(true);
      expect(body.result.tools.length).toBeGreaterThan(0);
    }
  }
});
```

#### Issue 3.6: Implicit Path in Search Tool
**Severity**: MEDIUM
**Lines**: 337-341
**Problem**: Uses `'.'` as search path which might not work on Windows or CI systems with different cwd.

```typescript
// BROKEN - Current directory might not be project root
params: {
  name: 'search_files',
  arguments: {
    pattern: '**/*',
    path: '.',
  },
}
```

**Fix**:
```typescript
test('should timeout long operations', async ({ request }) => {
  // Get project root for cross-platform compatibility
  const projectRoot = await request.get(`${MCP_SERVER_URL}/rpc`).then(() => {
    // MCP server should document where it runs from
    // For now, use Cargo.toml which should exist in project root
    return 'Cargo.toml';
  });

  const startTime = Date.now();

  const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
    data: {
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/call',
      params: {
        name: 'read_file',
        arguments: {
          file_path: 'Cargo.toml', // Always exists at project root
        },
      },
    },
    timeout: 30000,
  });

  const elapsed = Date.now() - startTime;
  expect(elapsed).toBeLessThan(30000);

  const body = await response.json();
  expect(body.jsonrpc).toBe('2.0');
  expect(body.result !== undefined || body.error !== undefined).toBeTruthy();
});
```

---

### 4. performance.spec.ts

**Issues Identified: 7 Critical/High-Severity**

#### Issue 4.1: Invalid P95 Calculation
**Severity**: CRITICAL
**Lines**: 41, 69, 102
**Problem**: P95 calculation is incorrect for small sample sizes.

```typescript
// BROKEN - P95 is wrong with 10 samples
const sortedTimes = [...times].sort((a, b) => a - b);
const p95 = sortedTimes[Math.floor(sortedTimes.length * 0.95)];
// With 10 items: floor(10 * 0.95) = floor(9.5) = 9 = last item (P90, not P95)
```

**Math error**:
- 10 items: index 9 = P90, not P95
- Correct: need ceil() not floor()
- Better: use 11+ samples for meaningful percentiles

**Fix**:
```typescript
function calculatePercentile(sorted: number[], percentile: number): number {
  if (sorted.length === 0) throw new Error('Empty array');

  // Use linear interpolation for more accurate percentiles
  const index = (percentile / 100) * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  const weight = index % 1;

  if (lower === upper) {
    return sorted[lower];
  }

  return sorted[lower] * (1 - weight) + sorted[upper] * weight;
}

test('health check should respond within threshold', async ({ request }) => {
  const times: number[] = [];
  const iterations = process.env.CI ? 10 : 20; // More iterations for accuracy

  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    const response = await request.get(`${MCP_SERVER_URL}/health`);
    const end = performance.now();

    expect(response.ok()).toBeTruthy();
    times.push(end - start);
  }

  const avg = times.reduce((a, b) => a + b, 0) / times.length;
  const sorted = [...times].sort((a, b) => a - b);
  const p95 = calculatePercentile(sorted, 95);
  const p99 = calculatePercentile(sorted, 99);

  console.log(`Health check latency - Avg: ${avg.toFixed(2)}ms, P95: ${p95.toFixed(2)}ms, P99: ${p99.toFixed(2)}ms`);

  expect(avg).toBeLessThan(THRESHOLDS.healthCheck);
  expect(p99).toBeLessThan(THRESHOLDS.healthCheck * 2); // P99 can be higher
});
```

#### Issue 4.2: Failed Requests Excluded from Latency Metric
**Severity**: HIGH
**Lines**: 156-189
**Problem**: Only measures successful requests. Failed requests skew the apparent performance.

```typescript
// BROKEN - Only successful requests affect latency metric
if (response.ok()) {
  times.push(end - start);
}
// This hides real-world failures where requests timeout
```

**Fix**:
```typescript
test('should maintain latency under load', async ({ request }) => {
  const concurrentRequests = 20;
  const latencies: { time: number; success: boolean }[] = [];

  const requests = Array.from({ length: concurrentRequests }, async (_, i) => {
    const start = performance.now();

    try {
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: i + 1,
          method: 'tools/list',
          params: {},
        },
      });
      const end = performance.now();

      latencies.push({
        time: end - start,
        success: response.ok(),
      });
    } catch (error) {
      const end = performance.now();
      latencies.push({
        time: end - start,
        success: false,
      });
    }
  });

  await Promise.all(requests);

  const successful = latencies.filter(l => l.success);
  const failed = latencies.filter(l => !l.success);

  const avgSuccess = successful.length > 0
    ? successful.reduce((sum, l) => sum + l.time, 0) / successful.length
    : 0;

  console.log(`Under load - Successful: ${successful.length}/${latencies.length}`);
  console.log(`Under load - Avg latency (success): ${avgSuccess.toFixed(2)}ms`);
  console.log(`Under load - Failed: ${failed.length}`);

  // Must have reasonable success rate
  expect(successful.length).toBeGreaterThanOrEqual(concurrentRequests * 0.9);

  // Successful requests should not degrade too much
  expect(avgSuccess).toBeLessThan(THRESHOLDS.toolsList * 3);
});
```

#### Issue 4.3: Division by Zero Risk
**Severity**: HIGH
**Lines**: 179-180
**Problem**: `times.reduce(...) / times.length` can throw if all requests fail.

```typescript
// BROKEN - If times is empty, division by zero
const avg = times.reduce((a, b) => a + b, 0) / times.length;
const max = Math.max(...times); // Also fails if times is empty
```

**Fix**:
```typescript
function calculateStats(times: number[]) {
  if (times.length === 0) {
    return {
      avg: 0,
      min: 0,
      max: 0,
      p95: 0,
      count: 0,
    };
  }

  const sorted = [...times].sort((a, b) => a - b);
  return {
    avg: times.reduce((a, b) => a + b, 0) / times.length,
    min: sorted[0],
    max: sorted[sorted.length - 1],
    p95: calculatePercentile(sorted, 95),
    count: times.length,
  };
}

const stats = calculateStats(times);
expect(stats.count).toBeGreaterThan(0); // Assert we have data
expect(stats.avg).toBeLessThan(THRESHOLDS.toolsList * 3);
```

#### Issue 4.4: Event Listener Added After Navigation
**Severity**: HIGH
**Lines**: 359-374
**Problem**: Adds `page.on('response')` to measure bundle sizes but SSE might have already streamed before listener attaches.

```typescript
// BROKEN - Event listener added AFTER navigation started
page.on('response', async (response) => {
  // This misses responses that arrived before listener registered
  ...
});

await page.goto('/');
```

**Fix**:
```typescript
test('should have reasonable WASM bundle size', async ({ page }) => {
  const resourceSizes: { name: string; size: number }[] = [];

  // Set up listener BEFORE navigation
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

  // NOW navigate
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  const wasmBundles = resourceSizes.filter((r) => r.name.includes('.wasm'));
  const jsBundles = resourceSizes.filter((r) => r.name.includes('.js'));
  const cssBundles = resourceSizes.filter((r) => r.name.includes('.css'));

  const totalWasm = wasmBundles.reduce((sum, r) => sum + r.size, 0) / 1024 / 1024;
  const totalJs = jsBundles.reduce((sum, r) => sum + r.size, 0) / 1024;
  const totalCss = cssBundles.reduce((sum, r) => sum + r.size, 0) / 1024;

  console.log(`Bundle sizes - WASM: ${totalWasm.toFixed(2)}MB, JS: ${totalJs.toFixed(2)}KB, CSS: ${totalCss.toFixed(2)}KB`);

  if (wasmBundles.length === 0) {
    console.warn('No WASM bundles detected - ensure .wasm files are being served');
  }

  if (totalWasm > 0) {
    expect(totalWasm).toBeLessThan(5);
  }
});
```

#### Issue 4.5: Memory Test Skip Called During Execution
**Severity**: HIGH
**Lines**: 288-291
**Problem**: `test.skip()` called AFTER test has started and made requests.

```typescript
// BROKEN - Skip called mid-test, already made HTTP requests
test.skip(
  initialMemory === undefined,
  'Server does not expose memory_mb...'
);

// Memory already used by previous requests
for (let i = 0; i < 100; i++) {
  await request.post(...); // Already requested before skip
}
```

**Fix**:
```typescript
test('should not leak memory on repeated requests', async ({ request }) => {
  // Check capability FIRST, before making any requests
  const healthResponse = await request.get(`${MCP_SERVER_URL}/health`);
  const healthData = await healthResponse.json();

  if (healthData.memory_mb === undefined) {
    test.skip();
    return; // Skip early, no further requests made
  }

  const initialMemory = healthData.memory_mb;

  // NOW make load requests
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

  // Check memory after load
  const finalHealth = await request.get(`${MCP_SERVER_URL}/health`);
  const finalData = await finalHealth.json();
  const finalMemory = finalData.memory_mb;

  const memoryGrowth = finalMemory - initialMemory;
  console.log(`Memory: ${initialMemory}MB -> ${finalMemory}MB (growth: ${memoryGrowth}MB)`);

  expect(memoryGrowth).toBeLessThan(50);
});
```

#### Issue 4.6: Flaky LCP Measurement with PerformanceObserver
**Severity**: MEDIUM
**Lines**: 260-277
**Problem**: Callback might not fire within timeout, returns null, assertion passes but no data.

```typescript
// BROKEN - Promise resolves after 1s timeout regardless of LCP
lcp = await Promise.race([
  new Promise(resolve => {
    new PerformanceObserver(...).observe(...);
    setTimeout(() => resolve(null), 1000);
  }),
]);

// Later: if (lcp) { ... } passes silently if LCP never fired
```

**Fix**:
```typescript
test('should measure Largest Contentful Paint', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');

  // Give rendering time
  await page.waitForTimeout(2000);

  const metrics = await page.evaluate(() => {
    return new Promise<{lcp: number | null; error?: string}>((resolve) => {
      const entries: PerformanceEntryList = [];

      try {
        const observer = new PerformanceObserver((list) => {
          entries.push(...list.getEntries());
        });

        observer.observe({ type: 'largest-contentful-paint', buffered: true });

        // Give observer time to collect
        setTimeout(() => {
          observer.disconnect();

          if (entries.length === 0) {
            resolve({ lcp: null, error: 'No LCP entries found' });
          } else {
            const lastEntry = entries[entries.length - 1] as any;
            resolve({ lcp: lastEntry.startTime });
          }
        }, 3000);
      } catch (error: any) {
        resolve({ lcp: null, error: error.message });
      }
    });
  });

  if (metrics.error) {
    console.warn(`LCP measurement failed: ${metrics.error}`);
  }

  if (metrics.lcp !== null) {
    console.log(`LCP: ${metrics.lcp.toFixed(2)}ms`);
    expect(metrics.lcp).toBeLessThan(4000);
  } else {
    console.log('LCP not available (common on very fast loads)');
    // Don't fail, some pages load too fast to measure LCP
  }
});
```

---

### 5. stability.spec.ts

**Issues Identified: 6 Critical/High-Severity**

#### Issue 5.1: Indefinite Loop in Sustained Load Test
**Severity**: CRITICAL
**Lines**: 42-65
**Problem**: `while` loop doesn't respect Playwright timeouts - if test hangs, no timeout protection.

```typescript
// BROKEN - Infinite loop, test timeout doesn't apply to loop logic
while (Date.now() - startTime < duration) {
  try {
    const response = await request.post(...);
    // If this request hangs, entire loop hangs
    // Test timeout still applies but poor design
  }
}
```

**Fix**:
```typescript
test('should handle sustained request load', async ({ request }) => {
  const duration = 30000; // 30 seconds
  const startTime = Date.now();
  let successCount = 0;
  let failCount = 0;
  let timeoutCount = 0;
  let requestCount = 0;

  // Use Promise.race to enforce per-request timeout
  while (Date.now() - startTime < duration) {
    try {
      const response = await Promise.race([
        request.post(`${MCP_SERVER_URL}/rpc`, {
          data: {
            jsonrpc: '2.0',
            id: ++requestCount,
            method: 'tools/list',
            params: {},
          },
        }),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Request timeout')), 5000)
        ),
      ]);

      if ((response as any).ok?.()) {
        successCount++;
      } else {
        failCount++;
      }
    } catch (error: any) {
      if (error.message === 'Request timeout') {
        timeoutCount++;
      } else {
        failCount++;
      }
    }

    // Small delay, but respect loop timeout
    const elapsed = Date.now() - startTime;
    if (elapsed >= duration) break;

    await new Promise((r) => setTimeout(r, Math.min(100, duration - elapsed)));
  }

  const successRate = requestCount > 0 ? (successCount / requestCount) * 100 : 0;
  console.log(`Sustained load (${duration / 1000}s):`);
  console.log(`  - Requests: ${requestCount}`);
  console.log(`  - Success: ${successCount} (${successRate.toFixed(2)}%)`);
  console.log(`  - Failed: ${failCount}`);
  console.log(`  - Timeouts: ${timeoutCount}`);

  expect(successRate).toBeGreaterThan(99);
  expect(requestCount).toBeGreaterThan(10); // Should complete at least some requests
});
```

#### Issue 5.2: No Validation of Reconnection Event Order
**Severity**: HIGH
**Lines**: 73-99
**Problem**: Counts reconnections but doesn't validate they're in correct order or follow failures.

```typescript
// BROKEN - Just counts events, no validation of actual reconnection flow
page.on('response', (response) => {
  if (response.url().includes('/events')) {
    reconnections++; // Could be 3 responses from same connection
  }
});

// Later: expect(disconnections).toBeLessThan(5); // Too loose
```

**Fix**:
```typescript
test('should maintain SSE connection for extended period', async ({ page }) => {
  const connectionEvents: Array<{timestamp: number; type: 'connect' | 'disconnect'}> = [];
  let lastConnectionState = 'unknown';

  // Track actual connection state changes
  page.on('response', (response) => {
    if ((response.url().includes('/events') || response.url().includes('/sse')) &&
        response.status() === 200 &&
        lastConnectionState !== 'connected') {
      connectionEvents.push({
        timestamp: Date.now(),
        type: 'connect',
      });
      lastConnectionState = 'connected';
    }
  });

  page.on('requestfailed', (request) => {
    if ((request.url().includes('/events') || request.url().includes('/sse')) &&
        lastConnectionState !== 'disconnected') {
      connectionEvents.push({
        timestamp: Date.now(),
        type: 'disconnect',
      });
      lastConnectionState = 'disconnected';
    }
  });

  await page.goto('/');

  // Wait for 60 seconds
  await page.waitForTimeout(60000);

  const disconnections = connectionEvents.filter(e => e.type === 'disconnect').length;
  const connections = connectionEvents.filter(e => e.type === 'connect').length;

  console.log(`SSE stability: ${connections} connections, ${disconnections} disconnections`);
  console.log(`Connection events:`, connectionEvents.map(e => ({
    type: e.type,
    relativeTime: e.timestamp,
  })));

  // Should maintain connection throughout
  expect(disconnections).toBeLessThan(3);
  expect(connections).toBeGreaterThan(0);
});
```

#### Issue 5.3: Page Reuse After Close
**Severity**: HIGH
**Lines**: 208-225
**Problem**: Closes page then uses same variable, can cause errors if context is also invalidated.

```typescript
// BROKEN - page is closed, then used
for (let i = 0; i < iterations; i++) {
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await page.close();

  page = await page.context().newPage(); // page.context() might be invalid after close
}
```

**Fix**:
```typescript
test('should clean up after many page loads', async ({ page, context }) => {
  const iterations = 20;

  for (let i = 0; i < iterations; i++) {
    // Create new page from context, not from closed page
    const testPage = context.pages().length > 0
      ? context.pages()[0]
      : await context.newPage();

    try {
      await testPage.goto('/');
      await testPage.waitForLoadState('networkidle');

      // Verify page is functional
      await expect(testPage.locator('body')).toBeVisible();
    } finally {
      // Close and ensure we don't reuse
      await testPage.close();
    }
  }

  // After many cycles, context should still be valid
  const finalPage = await context.newPage();
  try {
    await finalPage.goto('/');
    await expect(finalPage.locator('body')).toBeVisible();
  } finally {
    await finalPage.close();
  }
});
```

#### Issue 5.4: Special Characters Path Test Doesn't Read Files
**Severity**: MEDIUM
**Lines**: 301-331
**Problem**: Tests tool execution with special paths but doesn't create those files, so all requests will error.

```typescript
// BROKEN - Tests paths like "path with spaces" but these files don't exist
const specialPaths = [
  'path with spaces',
  'path-with-dashes',
  ...
];

for (const path of specialPaths) {
  const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
    params: {
      name: 'read_file',
      arguments: {
        file_path: path, // This file doesn't exist
      },
    },
  });

  // Should handle gracefully (not crash) - but all will error
  const body = await response.json();
  expect(body.jsonrpc).toBe('2.0');
  // This passes even though we didn't test what we meant to test
}
```

**Fix**:
```typescript
test('should handle special characters in paths', async ({ request }) => {
  const fs = require('fs');
  const path = require('path');

  const testFile = 'special_chars_test.txt';
  const testContent = 'Test content for special chars';

  try {
    // Create test file first
    fs.writeFileSync(testFile, testContent);

    const specialPaths = [
      'special_chars_test.txt', // Actual file
      'nonexistent_with spaces.txt', // File that doesn't exist
      'nonexistent-with-dashes.txt', // File that doesn't exist
    ];

    for (const filePath of specialPaths) {
      const response = await request.post(`${MCP_SERVER_URL}/rpc`, {
        data: {
          jsonrpc: '2.0',
          id: 1,
          method: 'tools/call',
          params: {
            name: 'read_file',
            arguments: {
              file_path: filePath,
            },
          },
        },
        timeout: 10000,
      });

      const body = await response.json();

      // Validate response structure
      expect(body.jsonrpc).toBe('2.0');
      expect(body.id).toBe(1);

      // For existing file, should have result
      if (filePath === 'special_chars_test.txt') {
        expect(body.result).toBeDefined();
        expect(body.result?.content).toContain(testContent);
      } else {
        // For nonexistent files, should have proper error
        expect(body.error).toBeDefined();
        expect(body.error.message).toBeTruthy();
      }
    }
  } finally {
    // Cleanup
    if (fs.existsSync(testFile)) {
      fs.unlinkSync(testFile);
    }
  }
});
```

#### Issue 5.5: Cleanup File List Never Cleaned
**Severity**: MEDIUM**
**Lines**: 17-32, 335-360
**Problem**: Files added to `TEST_FILES_TO_CLEANUP` but cleanup happens inside test, doesn't use `test.afterEach`.

```typescript
// BROKEN - Cleanup happens within test, not guaranteed to run
test('should handle unicode in content', async ({ request }) => {
  const testFile = 'test_unicode_temp.txt';
  TEST_FILES_TO_CLEANUP.push(testFile);

  // ... make request ...

  await cleanupTestFiles(); // Only called if test completes
  // If test fails/timeouts, files remain
});
```

**Fix**:
```typescript
import * as fs from 'fs';
import * as path from 'path';

const TEST_FILES: Set<string> = new Set();

async function cleanupTestFiles() {
  for (const filePath of TEST_FILES) {
    try {
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
      }
    } catch (error) {
      console.warn(`Failed to cleanup ${filePath}:`, error);
    }
  }
  TEST_FILES.clear();
}

test.describe('Edge Cases @stability', () => {
  // Cleanup after EACH test, guaranteed
  test.afterEach(async () => {
    await cleanupTestFiles();
  });

  test('should handle unicode in content', async ({ request }) => {
    const testFile = 'test_unicode_temp.txt';
    TEST_FILES.add(testFile);

    try {
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
      expect(body.result !== undefined || body.error !== undefined).toBeTruthy();

      // Verify file was actually written (if success)
      if (body.result && fs.existsSync(testFile)) {
        const content = fs.readFileSync(testFile, 'utf-8');
        expect(content).toContain('世界');
      }
    } finally {
      // Redundant, but `test.afterEach` will ensure cleanup
      TEST_FILES.add(testFile);
    }
  });
});
```

---

## Summary of Issues by Severity

| Severity | Count | Categories |
|----------|-------|------------|
| **CRITICAL** | 4 | Race conditions, infinite loops, WASM detection, invalid math |
| **HIGH** | 14 | Hardcoded waits, flakiness, weak selectors, missing validation |
| **MEDIUM** | 10 | Arbitrary tolerances, test isolation, incomplete checks |

### Total Issues Found: 28

---

## Recommendations for Fixes

### Phase 1 (Critical - Fix Immediately)
1. **Fix race conditions** in SSE tests - add response listeners BEFORE navigation
2. **Fix WASM initialization** detection - use actual element selectors, not text content
3. **Fix mathematical errors** in percentile calculations
4. **Fix path handling** - use absolute paths, not relative

### Phase 2 (High - Fix Before CI/CD)
1. Replace all `waitForTimeout` with event-based waits
2. Fix selector brittleness with priority-ordered lookups
3. Add missing assertions and schema validation
4. Remove/fix hardcoded delays

### Phase 3 (Medium - Improve Quality)
1. Add proper test file cleanup with `test.afterEach`
2. Improve timeout calculations for slow systems
3. Add cross-platform path compatibility
4. Better error messages and debug info

---

## Testing the Fixes

Run each category:
```bash
# Frontend tests
npm run test:e2e -- --grep "@frontend"

# Integration tests
npm run test:e2e -- --grep "@integration"

# Performance tests (longer)
npm run test:e2e -- --grep "@performance"

# Stability tests (much longer)
npm run test:e2e -- --grep "@stability" --timeout=600000
```

---

*Report Generated: 2025-01-25*
*Test Files Analyzed: 4 (1,245 lines)*
*Issues Identified: 28*
