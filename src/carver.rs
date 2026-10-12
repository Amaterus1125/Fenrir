// Signature-based file carving.
// Scans a raw image (disk image, loopback file, unallocated-space dump anything you can open() and read sequentially) for known magic-byte headers, and for formats with a matching footer/end-marker, carves out everything between header and footer as a candidate recovered file.
// Deliberately scoped: no filesystem metadata, no fragmentation reconstruction, no confidence scoring. If a file's blocks aren't contiguous on disk, this will produce a corrupt or truncated file, that's the known, documented limitation of this technique, not a bug.

// normal imports 
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct CarveHit {
 pub kind: &'static str,
 pub offset: u64,
 pub size: u64,
 pub output_path: PathBuf,
}

struct Signature {
 kind: &'static str,
 header: &'static [u8],
 footer: Option<&'static [u8]>,
 ext: &'static str,
 max_size: u64,
}

const SIGNATURES: &[Signature] = &[
 Signature { kind: "jpg", header: &[0xFF, 0xD8, 0xFF], footer: Some(&[0xFF, 0xD9]), ext: "jpg", max_size: 64 * 1024 * 1024 },
 Signature { kind: "png", header: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], footer: Some(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]), ext: "png", max_size: 64 * 1024 * 1024 },
 Signature { kind: "pdf", header: &[0x25, 0x50, 0x44, 0x46], footer: Some(&[0x25, 0x25, 0x45, 0x4F, 0x46]), ext: "pdf", max_size: 128 * 1024 * 1024 },
 // docx/xlsx/pptx are all ZIP containers andd PK\x03\x04 local file header.
 // We carve to the ZIP end-of-central-directory marker 
 Signature { kind: "docx/zip", header: &[0x50, 0x4B, 0x03, 0x04], footer: Some(&[0x50, 0x4B, 0x05, 0x06]), ext: "zip", max_size: 128 * 1024 * 1024 },
];

fn find_from(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
  if needle.is_empty() || start >= haystack.len() {
  return None;
    }
  haystack[start..]
  .windows(needle.len())
  .position(|w| w == needle)
  .map(|p| p + start)
}
/// Carve a raw byte buffer already loaded in memory (used for small images/ tests). For anything disk-sized, use carve_file_streaming instead.
pub fn carve_buffer(data: &[u8], out_dir: &Path) -> Result<Vec<CarveHit>, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("Cannot create output dir: {}", e))?;
    let mut hits = Vec::new();
    let mut hit_counter = 0usize;

    for sig in SIGNATURES {
     let mut search_from = 0usize;
      while let Some(start) = find_from(data, sig.header, search_from) {
      let footer_search_start = start + sig.header.len();
      let end = match sig.footer {
      Some(f) => find_from(data, f, footer_search_start).map(|p| p + f.len()),
     None => None,
      };
     let end = match end {
    Some(e) => e.min(start + sig.max_size as usize),
   None => (start + sig.max_size as usize).min(data.len()), // no footer found: cap-and-carve, marked truncated
   };
  let end = end.min(data.len());
   let slice = &data[start..end];
 hit_counter += 1;
  let out_name = format!("carved_{:04}_{}.{}", hit_counter, sig.kind.replace('/', "-"), sig.ext);
 let out_path = out_dir.join(&out_name);
  let mut f = File::create(&out_path).map_err(|e| format!("Cannot write {:?}: {}", out_path, e))?;
   f.write_all(slice).map_err(|e| format!("Cannot write {:?}: {}", out_path, e))?;
hits.push(CarveHit {
  kind: sig.kind,
   offset: start as u64,
    size: slice.len() as u64,
   output_path: out_path,
   });

search_from = end.max(start + 1);
  }}
 Ok(hits)
}
