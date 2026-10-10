<div align="center">

# FENRIR

**Devour the data. Hunt what's left. Keep the record.**

Secure erasure, signature-based file carving, and a tamper-evident audit log.
One static Rust binary, no C dependencies.

![Rust](https://img.shields.io/badge/rust-2021-orange?logo=rust)
![Linking](https://img.shields.io/badge/linking-musl%20static-success)
![C deps](https://img.shields.io/badge/C%20deps-none-blue)
![Platform](https://img.shields.io/badge/platform-linux-lightgrey?logo=linux)
![License](https://img.shields.io/badge/license-CC%20BY%204.0-informational)

</div>

---

In Norse myth, Fenrir is the wolf that swallows what it is given and leaves nothing behind. This tool does that, then does the opposite: it hunts through raw disk images for what was left behind, and it writes a hash-chained record of everything it did so nobody can quietly rewrite history.

Fenrir shares its checksum and atomic-write primitives with the **Chiral** package manager.

## Contents

- [Features](#features)
- [Install](#install)
- [Quick start](#quick-start)
- [Command reference](#command-reference)
- [How it works](#how-it-works)
- [Limitations](#limitations)
- [Roadmap](#roadmap)
- [Project layout](#project-layout)
- [Responsible use](#responsible-use)
- [License](#license)

## Features

| Module | Description |
|---|---|
| **Eraser** | Multi-pass overwrite, `fsync`, unlink, verify. DoD 5220.22-M (3-pass) or NIST 800-88 (1-pass random). |
| **Folder erase** | Recursive erase. Symlinks are unlinked, never followed, so it cannot escape the target tree. |
| **Drive-erase demo** | The same engine pointed at a loopback image. No physical disk is touched. |
| **Carver** | Recovers JPG, PNG, PDF and ZIP-based Office files from raw images by header/footer signature. |
| **Audit log** | Each operation is a JSONL entry that hashes in the previous entry. Edit or remove one line and every later hash fails. |

## Install

**Requirements:** Rust 1.70 or newer ([rustup.rs](https://rustup.rs)), Linux.

### Static musl build (recommended)

Fenrir has no C dependencies (`sha2`, `rand`, `chrono`, `serde` and `clap` are all pure Rust), so it links fully statically against musl. The resulting binary runs on minimal systems, containers, Alpine, LFS and rescue media with nothing installed.

```bash
git clone https://github.com/Amaterus1125/fenrir.git
cd fenrir

rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

sudo install -Dm755 target/x86_64-unknown-linux-musl/release/fenrir /usr/local/bin/fenrir
```

Confirm it is static:

```bash
file target/x86_64-unknown-linux-musl/release/fenrir
ldd  target/x86_64-unknown-linux-musl/release/fenrir    # "not a dynamic executable"
```

### Standard build

```bash
cargo build --release
sudo install -Dm755 target/release/fenrir /usr/local/bin/fenrir
```

This is **not** musl-linked on a glibc system. It links dynamically against your system libc. You only get a static binary by passing `--target ...-musl` (on a musl host such as Alpine, the default target is already musl).

For other architectures, use `aarch64-unknown-linux-musl` or `armv7-unknown-linux-musleabihf` with [`cross`](https://github.com/cross-rs/cross) or a matching musl toolchain.

### Cargo.toml

```toml
[package]
name = "fenrir"
version = "0.1.0"
edition = "2021"

[dependencies]
clap       = { version = "4", features = ["derive"] }
rand       = "0.8"
sha2       = "0.10"
chrono     = "0.4"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

The `[profile.release]` block is optional and gives a much smaller binary.

## Quick start

```bash
# Erase a file (3-pass DoD)
fenrir erase secrets.txt

# Erase a file with a single NIST pass (preferred for SSD/flash)
fenrir erase --nist old_keys.pem

# Wipe a directory tree
fenrir erase-folder ./scratch --nist

# Try the loopback demo (note: the image file is deleted afterwards)
dd if=/dev/urandom of=demo.img bs=1M count=16
fenrir drive-erase-demo demo.img

# Carve files out of a raw image
fenrir carve disk.img --out-dir recovered/

# Review and verify the audit trail
fenrir audit-show
fenrir audit-verify
```

Example output:

```text
✓ Erased "secrets.txt" — 4096 bytes, 3 pass(es) [DoD 5220.22-M (3-pass)], unlinked=true, sample-verify=true
✓ Audit chain intact — 3 entries, no tampering detected.
```

## Command reference

```text
fenrir [--audit-log <PATH>] <COMMAND>
```

| Command | Description |
|---|---|
| `erase <path> [--nist]` | Securely erase one file |
| `erase-folder <path> [--nist]` | Securely erase a directory tree, then remove it |
| `drive-erase-demo <image> [--nist]` | Erase a loopback image (demo only) |
| `carve <image> [--out-dir DIR]` | Carve files from a raw image (default `carved_output`) |
| `audit-show` | Print the audit log |
| `audit-verify` | Verify the hash chain |

`--audit-log` is global and defaults to `chiral-forensics-audit.log`. Exit code is `0` on success, `1` on error.

## How it works

### Eraser

1. `stat` the target; it must be a regular file.
2. For each pass: seek to 0, write 1 MiB chunks (zeros, `0xFF`, or CSPRNG output), then `sync_all()`.
3. Re-open and read back 8 random 4 KB sectors. If any is a run of identical bytes after a random pass, the overwrite probably did not land (copy-on-write filesystem, silent write failure) and `sample-verify=false` is reported.
4. Unlink, then confirm the path no longer resolves.

### Carver

Scans for each known header, then searches for the format's footer. A per-format size cap prevents a missing footer from carving gigabytes.

| Type | Header | Footer | Cap |
|---|---|---|---|
| JPG | `FF D8 FF` | `FF D9` | 64 MiB |
| PNG | `89 50 4E 47 0D 0A 1A 0A` | `IEND` + CRC | 64 MiB |
| PDF | `%PDF` | `%%EOF` | 128 MiB |
| ZIP / Office | `PK 03 04` | `PK 05 06` | 128 MiB |

### Audit chain

```text
entry_hash = SHA256( seq | timestamp | operation | target | details | prev_hash )
```

The first entry points at a genesis hash of 64 zeros. `audit-verify` recomputes every link and reports the first break. Writes go through temp file, `fsync`, then atomic `rename`, so a crash never leaves a half-written log.

## Limitations

Fenrir is deliberately scoped, and says so rather than overclaiming.

**Erasure**
- **SSDs and flash:** wear-leveling means overwriting a file cannot guarantee the same physical cells were hit. Use ATA Secure Erase, NVMe Sanitize, or full-disk encryption with key destruction for real sanitization.
- **Journaling and COW filesystems** (btrfs, ZFS, APFS, ext4 with data journaling) may keep old blocks elsewhere. Sample-verify catches some cases, not all.
- Backups, snapshots and filesystem metadata (names, sizes, timestamps) are not erased.
- `drive-erase-demo` is not a real device wipe. It overwrites and then **deletes** the image file.

**Carving**
- No filesystem awareness and no fragmentation handling. Fragmented files come out corrupt or truncated.
- JPEG footers can appear early (embedded thumbnails). ZIP carving stops at the end-of-central-directory signature and omits its trailing fields.
- The whole image is read into RAM. Large images need chunked scanning with boundary overlap.

**Audit log**
- Tamper-*evident*, not tamper-*proof*. An attacker with write access can truncate the tail or rebuild the entire chain. Anchor the latest hash somewhere external (print it, sign it, send it to another host) for stronger guarantees.
- Appends are read-modify-write. Do not run several instances against one log at the same time.

## Roadmap

-  Real block-device erase (`/dev/sdX`, discard, ATA/NVMe secure erase)
-  Streaming carver with chunk-overlap handling
-  More signatures (GIF, MP4, SQLite, ELF, 7z) and exact ZIP EOCD length
-  HMAC-signed audit entries and external hash anchoring
-  File locking for concurrent log writers
-  SHA-256 manifest for carved output

## Project layout

```text
src/
  main.rs     CLI (clap) and command dispatch
  eraser.rs   overwrite passes, verification, folder recursion
  carver.rs   signature table and carving logic
  audit.rs    hash-chained log, verify, print
  common.rs   sha256 helpers, atomic write/append (from Chiral PM)
```

## Responsible use

Erasure is irreversible. Double-check every path. Only run Fenrir against data and devices you own or are explicitly authorized to handle. The authors accept no liability for lost data.

## License

This work is licensed under the [Creative Commons Attribution 4.0 International License (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/).

You may share and adapt it, including commercially, as long as you give appropriate credit and indicate changes. See [`LICENSE`](LICENSE) for the full text.
