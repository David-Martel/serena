# Serena Dependencies - Quick Reference Guide

**TL;DR:** Score 72/100. Main issues: duplicate dependencies, outdated Leptos, deprecated serde_yaml.

## Immediate Actions (30 minutes)

```bash
# Windows
scripts\fix-dependencies.bat

# Linux/Mac
chmod +x scripts/fix-dependencies.sh
./scripts/fix-dependencies.sh
```

## Top 5 Issues to Fix

### 1. Fix cargo-audit (5 min)
```bash
cargo install cargo-audit --force
cargo audit
```

### 2. Update Patch Versions (2 min)
```bash
cargo update rustix tempfile
```

### 3. Fix Dashboard Profile Warning (10 min)
**Problem:** Profile in crates/serena-dashboard/Cargo.toml is ignored

**Fix:** Move to workspace root
```toml
# In T:\projects\serena-source\Cargo.toml, add:
[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```

Then delete the `[profile.release]` section from `crates/serena-dashboard/Cargo.toml`.

### 4. Replace Deprecated serde_yaml (30 min)

**Files to update:**
- `crates/serena/Cargo.toml`
- `crates/serena-config/Cargo.toml`
- `crates/serena-memory/Cargo.toml`

**Change:**
```toml
# OLD
serde_yaml = { workspace = true }

# NEW (option 1 - maintained fork)
serde_yml = "0.0.10"

# NEW (option 2 - switch to TOML)
toml = { workspace = true }  # Already in workspace deps
```

**Code changes:**
```rust
// OLD
use serde_yaml;
let data: Config = serde_yaml::from_str(&content)?;

// NEW (option 1)
use serde_yml;
let data: Config = serde_yml::from_str(&content)?;

// NEW (option 2 - TOML)
use toml;
let data: Config = toml::from_str(&content)?;
```

### 5. Optimize tokio Features (15 min)

**Current (workspace):**
```toml
tokio = { version = "1.41", features = ["full"] }
```

**Replace in each crate:**

```toml
# crates/serena-core/Cargo.toml
tokio = { workspace = true, features = ["io-std", "io-util", "sync"] }

# crates/serena-mcp/Cargo.toml
tokio = { workspace = true, features = ["io-std", "io-util", "sync", "macros"] }

# crates/serena-web/Cargo.toml
tokio = { workspace = true, features = ["io-std", "io-util", "sync", "macros", "rt-multi-thread"] }

# crates/serena/Cargo.toml
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "signal"] }
```

**Workspace update:**
```toml
# Remove "full", make it minimal
tokio = { version = "1.41" }  # No default features
```

**Savings:** ~500 KB binary size

## Critical Duplicates

| Crate | Versions | Impact | Fix Timeline |
|-------|----------|--------|--------------|
| **leptos** | 0.6.15 | Dashboard rebuild needed | Next sprint |
| **dashmap** | 5.5.3, 6.1.0 | 2x memory overhead | This week |
| **http** | 0.2, 1.4 | Type incompatibility | Wait for ecosystem |
| **hyper** | 0.14, 1.8 | Async differences | Wait for reqwest |
| **thiserror** | 1.0, 2.0 | Error API changes | Pin v1.0 |
| **windows-sys** | 4 versions | Windows bloat | Ecosystem issue |

## Leptos 0.8 Migration Plan

**Current:** 0.6.15
**Latest:** 0.8.15
**Effort:** 1-2 days
**Breaking Changes:** Yes (reactive system refactor)

**Affected crates:**
- serena-dashboard (main work)

**Dependencies to update:**
- leptos: 0.6.15 → 0.8.15
- leptos_config: 0.6.15 → 0.8.8
- leptos_dom: 0.6.15 → 0.8.7
- leptos_macro: 0.6.15 → 0.8.14
- server_fn: 0.6.15 → 0.8.9

**Migration steps:**
1. Read Leptos 0.8 migration guide
2. Update Cargo.toml versions
3. Fix compilation errors (expect signal API changes)
4. Test WASM build: `wasm-pack build --target web`
5. Update dashboard UI code
6. Test in browser

## Quick Health Check Commands

```bash
# Current duplicates
cargo tree --duplicates --workspace

# Outdated dependencies
cargo outdated --workspace

# Security audit
cargo audit

# Build size analysis
cargo bloat --release --crates

# Dependency count
cargo tree --workspace | wc -l

# Why is X included?
cargo tree --workspace -i <crate-name>
```

## Workspace Standardization

**Good practices already in place:**
- ✅ Workspace dependencies
- ✅ Consistent rust-version (1.75)
- ✅ Build profiles for optimization
- ✅ Modular crate structure

**Needs improvement:**
- ⚠️ Feature flag optimization (tokio "full")
- ⚠️ Dev dependency versions (tempfile 3.8 vs 3.10)
- ⚠️ Profile placement (dashboard)

## Success Criteria

**After immediate fixes (this week):**
- [ ] cargo-audit runs without errors
- [ ] No profile warnings
- [ ] Patch versions updated
- [ ] Build size reduced by 1-2 MB

**After short-term fixes (next sprint):**
- [ ] serde_yaml replaced
- [ ] tokio features optimized
- [ ] once_cell removed
- [ ] Score: 82/100

**After medium-term fixes (next release):**
- [ ] Leptos 0.8 migration complete
- [ ] <5 duplicate dependencies
- [ ] Score: 90/100

## Resources

- Full report: `DEPENDENCY_HEALTH_REPORT.md`
- Leptos migration: https://github.com/leptos-rs/leptos/blob/main/docs/migration/0.7.md
- Cargo book: https://doc.rust-lang.org/cargo/
- Dependency management: https://rust-lang.github.io/api-guidelines/

## Questions?

- "Why so many duplicates?" → Ecosystem transition (hyper 0.14→1.x)
- "Is this urgent?" → No security issues, but affects binary size/performance
- "When to update?" → Immediate fixes now, major updates next sprint
- "Will it break things?" → Not if you follow the steps above

---

**Last updated:** 2025-12-25
**Health score:** 72/100
**Run:** `scripts/fix-dependencies.bat` (Windows) or `scripts/fix-dependencies.sh` (Linux/Mac)
