# Heartbeat Fix Testing Guide

## Quick Test Steps

### 1. Test Normal Operation
```bash
# Start Serena server
cd C:\codedev\serena-mcp
# Your normal start command here

# Open dashboard in browser
# Check browser console (F12):
# Should see: "Heartbeat started with 5000ms interval"
```

### 2. Test Connection Loss Detection
```bash
# While dashboard is open:
# 1. Stop the Serena backend server
# 2. Wait and observe the dashboard

# Expected behavior:
# - After ~10 seconds (2 failures): Yellow warning banner appears
# - After ~25 seconds (5 failures): Red error banner appears
# - Console shows: "Heartbeat failure threshold reached (5 failures)"
# - Dashboard remains open and functional
```

### 3. Test Automatic Reconnection
```bash
# After connection loss is detected:
# 1. Restart the Serena backend server
# 2. Observe automatic reconnection

# Expected behavior:
# - Reconnection attempts logged to console
# - Exponential backoff delays: 2s, 4s, 8s, 16s, 30s...
# - On success: "Reconnection successful!" in console
# - Error banners disappear automatically
# - Page data reloads
```

### 4. Test Manual Retry
```bash
# While connection error is showing:
# 1. Click "Retry Now" button

# Expected behavior:
# - Reconnection counter resets to 0
# - Immediate reconnection attempt
# - Status updates in real-time
```

### 5. Test Reload Dashboard
```bash
# While connection error is showing:
# 1. Click "Reload Dashboard" button

# Expected behavior:
# - Full page reload
# - Fresh dashboard load
```

## Visual Indicators

### Warning State (2 failures)
```
┌────────────────────────────────────────────────────┐
│ ⚠️ Connection Warning: Server heartbeat is         │
│ experiencing issues. Monitoring connection...      │
└────────────────────────────────────────────────────┘
```

### Error State (5+ failures)
```
┌────────────────────────────────────────────────────┐
│ 🔴 Connection Lost: Cannot reach Serena server.    │
│ Attempting to reconnect...                         │
│                                                    │
│  [Retry Now]  [Reload Dashboard]                  │
└────────────────────────────────────────────────────┘
```

## Console Messages to Expect

### Normal Operation
```
Heartbeat started with 5000ms interval
```

### Connection Issues
```
Heartbeat failure #1: error - Failed to fetch
Heartbeat failure #2: error - Failed to fetch
Heartbeat failure threshold reached (5 failures)
Reconnection attempt #1 in 2 seconds...
Attempting reconnection now...
```

### Successful Recovery
```
Reconnection successful!
Heartbeat recovered - connection restored
```

## Regression Tests

### Before Fix Behavior (BROKEN)
- ❌ Dashboard closed after 250ms
- ❌ No error message shown
- ❌ No reconnection attempt
- ❌ No user control

### After Fix Behavior (WORKING)
- ✅ Dashboard stays open
- ✅ Clear error messages
- ✅ Automatic reconnection
- ✅ Manual retry option
- ✅ 95% less network traffic

## Performance Verification

### Network Tab (F12 → Network)
**Before:**
- 4 heartbeat requests per second
- Immediate failure on any 500ms delay

**After:**
- 1 heartbeat request every 5 seconds
- Tolerates up to 50 seconds of failures (5 failures × 10s timeout)

## Troubleshooting

### Dashboard still closing immediately?
- Clear browser cache (Ctrl+Shift+Del)
- Hard reload (Ctrl+F5)
- Check that dashboard.js was updated (check file size: should be 2120 lines)

### Reconnection not working?
- Check backend server is actually running
- Verify /heartbeat endpoint is responding
- Check browser console for detailed error messages

### Exponential backoff not visible?
- Reconnection happens quickly if server starts immediately
- Test with longer server downtime (30+ seconds) to see backoff progression

## Success Criteria

✅ Dashboard does not close on connection loss
✅ Warning appears after ~10 seconds
✅ Error appears after ~25 seconds
✅ Automatic reconnection works
✅ Manual retry button works
✅ Reload button works
✅ Banners disappear on successful reconnection
✅ No JavaScript errors in console
✅ Heartbeat interval is 5 seconds (not 250ms)

---

**Test Date:** _______________
**Tester:** _______________
**Result:** ☐ PASS ☐ FAIL
**Notes:** _______________
