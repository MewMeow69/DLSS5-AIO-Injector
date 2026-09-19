use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub fn read_exact_at(f: &mut File, off: u64, len: usize) -> Option<Vec<u8>> {
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut b = vec![0u8; len];
    f.read_exact(&mut b).ok()?;
    Some(b)
}

fn read_cstr(f: &mut File, off: u64, cap: usize) -> Option<String> {
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut out = Vec::new();
    let mut buf = [0u8; 1];
    for _ in 0..cap {
        if f.read(&mut buf).ok()? == 0 || buf[0] == 0 {
            break;
        }
        out.push(buf[0]);
    }
    Some(String::from_utf8_lossy(&out).to_string())
}

pub struct Pe {
    pub machine: u16,
    pub imports: Vec<String>,
}

impl Pe {
    pub fn is_x64(&self) -> bool {
        match self.machine {
            0x8664 | 0xAA64 => true,
            0x14c => false,
            _ => true,
        }
    }

    pub fn has_import(&self, name: &str) -> bool {
        self.imports.iter().any(|i| i == name)
    }
}

// ponytail: hand-rolled PE reader, header + import table only. Swap for the `object` crate if
// resource/version parsing is ever needed.
pub fn pe_read(path: &Path) -> Option<Pe> {
    let mut f = File::open(path).ok()?;
    let dos = read_exact_at(&mut f, 0, 0x40)?;
    if &dos[0..2] != b"MZ" {
        return None;
    }
    let pe_off = u32::from_le_bytes(dos[0x3C..0x40].try_into().ok()?) as u64;
    let coff = read_exact_at(&mut f, pe_off, 0x18)?;
    if &coff[0..4] != b"PE\0\0" {
        return None;
    }
    let machine = u16::from_le_bytes(coff[4..6].try_into().ok()?);
    let nsects = u16::from_le_bytes(coff[6..8].try_into().ok()?) as usize;
    let size_opt = u16::from_le_bytes(coff[0x14..0x16].try_into().ok()?) as usize;
    let opt = read_exact_at(&mut f, pe_off + 0x18, size_opt)?;
    if opt.len() < 96 || nsects == 0 || nsects > 128 {
        return None;
    }
    let magic = u16::from_le_bytes(opt[0..2].try_into().ok()?);
    let dd_off = if magic == 0x20B { 112 } else { 96 };
    if opt.len() < dd_off + 16 * 8 {
        return None;
    }

    let mut sects: Vec<(u32, u32, u32, u32)> = Vec::with_capacity(nsects);
    let sects_off = pe_off + 0x18 + size_opt as u64;
    for i in 0..nsects {
        let s = read_exact_at(&mut f, sects_off + (i * 40) as u64, 40)?;
        let vsize = u32::from_le_bytes(s[8..12].try_into().ok()?);
        let va = u32::from_le_bytes(s[12..16].try_into().ok()?);
        let rawsize = u32::from_le_bytes(s[16..20].try_into().ok()?);
        let raw = u32::from_le_bytes(s[20..24].try_into().ok()?);
        sects.push((va, vsize, raw, rawsize));
    }

    let rva2off = |rva: u32| -> Option<u64> {
        for &(va, vsize, raw, rawsize) in &sects {
            let v = if vsize == 0 { rawsize } else { vsize };
            if rva >= va && rva < va.saturating_add(v) {
                return Some(raw as u64 + (rva - va) as u64);
            }
        }
        None
    };

    let mut imports = Vec::new();
    let mut collect = |dir_off: usize, f: &mut File| {
        let rva = u32::from_le_bytes(opt[dir_off..dir_off + 4].try_into().unwrap_or([0; 4]));
        if rva == 0 {
            return;
        }
        let Some(base) = rva2off(rva) else { return };
        for i in 0..1024u64 {
            let Some(d) = read_exact_at(f, base + i * 20, 20) else {
                break;
            };
            if d.iter().all(|b| *b == 0) {
                break;
            }
            let name_rva = u32::from_le_bytes(d[12..16].try_into().unwrap_or([0; 4]));
            if let Some(o) = rva2off(name_rva) {
                if let Some(n) = read_cstr(f, o, 260) {
                    let n = n.to_lowercase();
                    if !n.is_empty() && !imports.contains(&n) {
                        imports.push(n);
                    }
                }
            }
        }
    };
    collect(dd_off + 8, &mut f); // import directory
    collect(dd_off + 13 * 8, &mut f); // delay import directory

    Some(Pe { machine, imports })
}

fn scan_buf(buf: &[u8], markers: &[&str]) -> Vec<String> {
    let mut hay = buf.to_vec();
    hay.make_ascii_lowercase();
    let mut hits = Vec::new();
    for m in markers {
        let n: Vec<u8> = m.bytes().map(|b| b.to_ascii_lowercase()).collect();
        if n.is_empty() || hay.len() < n.len() {
            continue;
        }
        let first = n[0];
        let found = hay.iter().enumerate().any(|(i, &b)| {
            if b != first || i + n.len() > hay.len() {
                return false;
            }
            hay[i..i + n.len()] == n[..]
        }) || {
            let last = hay.len().saturating_sub(n.len() * 2);
            (0..last).any(|i| {
                hay[i] == first
                    && hay[i + 1] == 0
                    && (1..n.len()).all(|j| hay[i + j * 2] == n[j] && hay[i + j * 2 + 1] == 0)
            })
        };
        if found {
            hits.push(m.to_string());
        }
    }
    hits
}

pub fn scan_file_markers(path: &Path, markers: &[&str], cap: u64) -> Vec<String> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Vec::new();
    };
    let len = meta.len().min(cap) as usize;
    if len == 0 {
        return Vec::new();
    }
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let mut buf = vec![0u8; len];
    if f.read(&mut buf).is_err() {
        return Vec::new();
    }
    scan_buf(&buf, markers)
}

#[cfg(test)]
pub fn file_has_marker(path: &Path, marker: &str, cap: u64) -> bool {
    !scan_file_markers(path, &[marker], cap).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_system_dll_imports() {
        let sys = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let path = Path::new(&sys).join("System32").join("kernel32.dll");
        let pe = pe_read(&path).expect("kernel32 parses");
        assert!(pe.is_x64());
        assert!(!pe.imports.is_empty());
    }

    #[test]
    fn finds_markers_ascii_and_utf16() {
        let dir = std::env::temp_dir().join("neurodeck-marker-test");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("ascii.bin");
        let mut f = std::fs::File::create(&a).unwrap();
        f.write_all(b"....nvngx_dlss.dll....").unwrap();
        drop(f);
        assert_eq!(scan_file_markers(&a, &["nvngx_dlss", "libxess"], 1 << 20), vec!["nvngx_dlss".to_string()]);

        let b = dir.join("utf16.bin");
        let mut raw = b"N\0V\0N\0G\0X\0_\0D\0L\0S\0S\0G\0".to_vec();
        raw.extend_from_slice(b"\0\0\0\0");
        std::fs::write(&b, raw).unwrap();
        assert!(file_has_marker(&b, "nvngx_dlssg", 1 << 20));
        std::fs::remove_dir_all(&dir).ok();
    }
}
