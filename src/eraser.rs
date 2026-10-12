// Secure file/folder eraser -----
// Pass patterns follow DoD 5220.22-M (3-pass: zeros, ones, random) as the default, with a NIST 800-88 "single-pass random" mode for modern flash media where multi-pass overwrite is mostly theater (SSD wear-leveling
// means you can't guarantee you overwrote the same physical cells anyway this tool is honest about that in its output rather than pretending otherwise).
// Every pass is followed by fsync. After the final pass the file is unlinked, then we verify: (a) the path no longer resolves, and (b) for the overwrite itself, we re-open the file *during* the last pass and
// read back a sample of sectors to confirm the on-disk bytes are what we just wrote, not stale original data (catches silent write failures/COW filesystems that didn't actually overwrite in place).

