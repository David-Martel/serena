# E2E Test Quick Fixes - One Liners

Copy-paste solutions for the top issues.

## Critical Issues (Fix First)

### 1. Path Must Be Absolute (playwright.config.ts:121)

```typescript
// BEFORE
cwd: '..',

// AFTER
import * as path from 'path';
cwd: path.resolve(__dirname, '..'),
```

### 2. WASM Init Detection (frontend.spec.ts:14-21)

```typescript
// BEFORE
await page.waitForFunction(() => {
  return document.body.textContent !== '' &&
    !document.querySelector('noscript');
}, { timeout: 30000 });

// AFTER
await page.waitForSelector('[data-testid="dashboard"]', { timeout: 10000 })
  .catch(() => page.waitForSelector('#app', { timeout: 10000 }))
  .catch(() => page.waitForFunction(
    () => document.querySelectorAll('[data-tool], .tool-card').length > 0,
    { timeout: 10000 }
  ));
```

### 3. SSE Race Condition (integration.spec.ts:36-50)

```typescript
// BEFORE - Sets up listener AFTER goto
const ssePromise = page.waitForResponse(...);
await page.goto('/');
const sseResponse = await ssePromise;

// AFTER - Sets up listener BEFORE goto
let sseConnected = false;
page.on('response', (r) => {
  if (r.url().includes('/sse') && r.headers()['content-type']?.includes('text/event-stream')) {
    sseConnected = true;
  }
});
await page.goto('/');
await page.waitForFunction(() => sseConnected, { timeout: 10000 });
```

### 4. P95 Calculation (performance.spec.ts:41)

```typescript
// BEFORE - WRONG MATH
const p95 = sortedTimes[Math.floor(sortedTimes.length * 0.95)];
// With 10 items: floor(9.5) = 9 = last item (actually P90)

// AFTER - CORRECT
function calcPercentile(arr: number[], p: number) {
  const idx = (p / 100) * (arr.length - 1);
  const lower = Math.floor(idx);
  const upper = Math.ceil(idx);
  const weight = idx % 1;
  return arr[lower] * (1 - weight) + arr[upper] * weight;
}
const p95 = calcPercentile(sortedTimes, 95);
```

---

## High Severity Issues

### 5. Remove All Hardcoded Waits

```typescript
// BEFORE - BAD
await page.waitForTimeout(2000);

// AFTER - GOOD
await page.waitForFunction(
  () => actualCondition,
  { timeout: 10000 }
);
```

### 6. Multi-Selector Should Check One at a Time

```typescript
// BEFORE - Fragile
const el = page.getByTestId('status')
  .or(page.locator('.status'))
  .first();

// AFTER - Robust
async function findElement(page, selectors) {
  for (const selector of selectors) {
    const el = page.locator(selector);
    if (await el.count() > 0) return el.first();
  }
  throw new Error(`Not found: ${selectors}`);
}
const el = await findElement(page, ['[data-testid="status"]', '.status']);
```

### 7. Handle Failed Requests in Perf Tests

```typescript
// BEFORE - Only successful requests count
if (response.ok()) times.push(elapsed);

// AFTER - Track both success and failure
results.push({ success: response.ok(), time: elapsed });
const successful = results.filter(r => r.success);
expect(successful.length / results.length).toBeGreaterThan(0.9);
const avgLatency = successful.reduce((s,r) => s + r.time, 0) / successful.length;
```

### 8. No Infinite Loops in Tests

```typescript
// BEFORE - Can hang forever
while (Date.now() - start < duration) {
  await request.post(...); // If this hangs, loop hangs
}

// AFTER - Has timeout per request
while (Date.now() - start < duration) {
  const elapsed = Date.now() - start;
  if (elapsed >= duration) break;

  try {
    await Promise.race([
      request.post(...),
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error('timeout')), 5000)
      )
    ]);
  } catch (error) {
    // Handle timeout
  }
}
```

---

## Medium Severity Issues

### 9. File Cleanup Should Use test.afterEach

```typescript
// BEFORE - Not guaranteed to run
async function cleanupFiles() { /* ... */ }
TEST_FILES.push(file);
// ... test code ...
await cleanupFiles(); // Only runs if test completes successfully

// AFTER - Always runs
const TEST_FILES = new Set<string>();

test.describe('...', () => {
  test.afterEach(async () => {
    for (const f of TEST_FILES) fs.existsSync(f) && fs.unlinkSync(f);
    TEST_FILES.clear();
  });

  test('...', async () => {
    TEST_FILES.add(file);
    // ... test code ...
  });
});
```

### 10. Don't Skip Tests Mid-Execution

```typescript
// BEFORE - Skip called after requests made
const memory = await request.get('/health').then(r => r.json());
test.skip(memory.memory_mb === undefined, 'No memory metric');
for (let i = 0; i < 100; i++) {
  await request.post(...); // Already made requests before skip
}

// AFTER - Check and skip FIRST
const memory = await request.get('/health').then(r => r.json());
if (memory.memory_mb === undefined) {
  test.skip();
  return;
}
// NOW make load requests
for (let i = 0; i < 100; i++) {
  await request.post(...);
}
```

---

## Checklist: Did You Fix It?

Run this to test each category:

```bash
# Frontend (15 tests, should complete in <30s)
npm run test:e2e -- --grep "@frontend"

# Integration (11 tests, should complete in <2m)
npm run test:e2e -- --grep "@integration"

# Performance (8 tests, should complete in <5m)
npm run test:e2e -- --grep "@performance"

# Stability (9 tests, should complete in <10m)
npm run test:e2e -- --grep "@stability"
```

**Success Criteria:**
- ✅ No "timeout" or "test timeout" errors
- ✅ No "Flaky" test reports
- ✅ No race condition failures (tests fail inconsistently)
- ✅ All assertions pass in clean environment

---

## Files Modified

1. `playwright.config.ts` - 2 changes (path handling)
2. `frontend.spec.ts` - 8 changes (WASM, selectors, timeouts, assertions)
3. `integration.spec.ts` - 6 changes (race conditions, validation, paths)
4. `performance.spec.ts` - 7 changes (math, latency, memory, skip logic)
5. `stability.spec.ts` - 6 changes (loops, cleanup, validation, state tracking)

**Total Lines Changed:** ~150 lines across 5 files

---

*For detailed explanations, see E2E_TEST_REVIEW.md*
*For line-by-line changes, see FIXES.md*
