// Tamper-evident audit log: every operation (erase, carve, drive-erase) appends one JSON line. Each entry hashes in the previous entry's hash,
// so altering or deleting any past line breaks every hash after it same chaining idea as the PM's checksum verification, just applied to a sequence of events instead of a single file.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::common::{append_atomic, sha256_hex_of_bytes};
