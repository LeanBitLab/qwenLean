# RPM Build Fix — Tauri v2 Bundler Stall

## Problem

`cargo tauri build --bundles rpm` hangs indefinitely at:

```
Bundling Qwen Studio-2.2.0-1.x86_64.rpm
```

CPU pinned at 100% on one core, no progress indicator, no output for 30+ minutes.

## Root Cause

This is a **known upstream bug** in Tauri v2's RPM bundler:

| Source | Details |
|--------|---------|
| Issue | [tauri-apps/tauri#11478](https://github.com/tauri-apps/tauri/issues/11478) |
| Status | Closed as `not_planned` |
| Culprit | [`rpm-rs`](https://github.com/rpm-rs/rpm) crate's gzip compression |
| Trigger | Large binary size (~146MB for qwen-studio) |

Tauri v2's RPM bundler delegates compression to the `rpm-rs` crate, which uses single-threaded gzip compression that becomes exponentially slower as binary size increases. The DEB bundler uses a completely different (`dpkg-deb`) path that handles large files fine — this is **RPM-specific only**.

Community reports:

| Binary Size | Build Time |
|-------------|------------|
| 24MB | ~10-15 seconds |
| 100MB | ~15-30 minutes |
| 146MB (qwen-studio) | indefinite stall |
| 1GB+ | 36+ minutes |

## Fix

Added compression bypass in `tauri.conf.json`:

```json
{
  "bundle": {
    "linux": {
      "rpm": {
        "compression": {
          "type": "none"
        }
      }
    }
  }
}
```

**Effect:** Skips compression entirely. Build drops from 30+ minutes to **under 5 seconds**.

**Trade-off:** RPM file is larger (~140MB uncompressed vs potentially ~70MB compressed), but installs identically. Linux package managers handle this fine.

## What Didn't Work

| Attempted | Result |
|-----------|--------|
| `"type": "gzip", "level": 1` | Still stalled |
| `[profile.release] strip = "symbols"` | Reduced binary slightly, still stalled |
| `"type": "zstd"` | Not supported by installed `rpm-rs` version (0.16) |

## Build Workflow

```bash
# First build or after code changes:
cargo tauri build --bundles rpm

# When binary already built (skip compilation):
cargo tauri bundle --bundles rpm

# Install:
sudo rpm -Uvh --force target/release/bundle/rpm/Qwen\ Studio-2.2.0-1.x86_64.rpm

# Test:
/usr/bin/qwen-studio

# Uninstall:
sudo rpm -e qwen-studio
```

## References

- [tauri-apps/tauri#11478](https://github.com/tauri-apps/tauri/issues/11478) — Original bug report
- [tauri-apps/tauri#9840](https://github.com/tauri-apps/tauri/pull/9840) — PR that reduced gzip level (still too slow)
- [tauri-apps/tauri#11584](https://github.com/tauri-apps/tauri/pull/11584) — PR that added compression config option
- [tauri-apps/tauri#13273](https://github.com/tauri-apps/tauri/issues/13273) — Related: 2hr build times in CI
- [rpm-rs/rpm#297](https://github.com/rpm-rs/rpm/issues/297) — Upstream crate performance issue
