// Signature-based file carving.
// Scans a raw image (disk image, loopback file, unallocated-space dump anything you can open() and read sequentially) for known magic-byte headers, and for formats with a matching footer/end-marker, carves out everything between header and footer as a candidate recovered file.
// Deliberately scoped: no filesystem metadata, no fragmentation reconstruction, no confidence scoring. If a file's blocks aren't contiguous on disk, this will produce a corrupt or truncated file, that's the known, documented limitation of this technique, not a bug.

