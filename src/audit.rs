// Tamper-evident audit log: every operation (erase, carve, drive-erase) appends one JSON line. Each entry hashes in the previous entry's hash,
// so altering or deleting any past line breaks every hash after it same chaining idea as the PM's checksum verification, just applied to a sequence of events instead of a single file.
