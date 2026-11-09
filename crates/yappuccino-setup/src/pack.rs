//! The installer's own archive. `yappuccino-pack` appends it to a copy of the
//! setup program; the setup program finds it in itself by its trailer:
//!
//! ```text
//! stub exe | payload | payload length (u64 LE) | b"YAPSETUP"
//! payload:  b"YAPPACK1" | version (u32 len + UTF-8) | total raw bytes (u64)
//!           | file count (u32) | files
//! file:     path (u32 len + UTF-8, '/'-separated) | raw length (u64)
//!           | deflated length (u64) | deflated bytes
//! ```
//!
//! Every number is little-endian. The stub alone is what goes into the
//! install folder as the uninstaller.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};

const TRAILER: &[u8; 8] = b"YAPSETUP";
const MAGIC: &[u8; 8] = b"YAPPACK1";

/// An archive read out of a setup program, and where its stub ends.
pub struct Payload {
    pub version: String,
    pub total: u64,
    pub stub_len: u64,
    bytes: Vec<u8>,
}

fn bad(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

/// Packs every file under `dir`, but the ZIP's README.txt, into an archive.
pub fn pack(dir: &Path, version: &str) -> io::Result<Vec<u8>> {
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    let mut entries = Vec::new();
    let mut total = 0u64;
    for name in &files {
        let raw = fs::read(dir.join(name))?;
        total += raw.len() as u64;
        let deflated = miniz_oxide::deflate::compress_to_vec(&raw, 9);
        put_str(&mut entries, name);
        entries.extend_from_slice(&(raw.len() as u64).to_le_bytes());
        entries.extend_from_slice(&(deflated.len() as u64).to_le_bytes());
        entries.extend_from_slice(&deflated);
    }
    let mut out = MAGIC.to_vec();
    put_str(&mut out, version);
    out.extend_from_slice(&total.to_le_bytes());
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    out.extend_from_slice(&entries);
    Ok(out)
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if name != "README.txt" {
                files.push(name);
            }
        }
    }
    Ok(())
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// Writes `stub` with `payload` and its trailer after it.
pub fn write_setup(stub: &[u8], payload: &[u8], out: &Path) -> io::Result<()> {
    let mut file = io::BufWriter::new(fs::File::create(out)?);
    file.write_all(stub)?;
    file.write_all(payload)?;
    file.write_all(&(payload.len() as u64).to_le_bytes())?;
    file.write_all(TRAILER)?;
    file.flush()
}

/// The archive inside the setup program at `exe`, if it carries one.
pub fn read(exe: &Path) -> io::Result<Option<Payload>> {
    let mut file = fs::File::open(exe)?;
    let size = file.metadata()?.len();
    if size < 16 {
        return Ok(None);
    }
    let mut tail = [0u8; 16];
    file.seek(SeekFrom::Start(size - 16))?;
    file.read_exact(&mut tail)?;
    if &tail[8..] != TRAILER {
        return Ok(None);
    }
    let len = u64::from_le_bytes(tail[..8].try_into().unwrap());
    let stub_len = size
        .checked_sub(16 + len)
        .ok_or_else(|| bad("payload longer than the file"))?;
    let mut bytes = vec![0u8; len as usize];
    file.seek(SeekFrom::Start(stub_len))?;
    file.read_exact(&mut bytes)?;
    parse(bytes, stub_len).map(Some)
}

fn parse(bytes: Vec<u8>, stub_len: u64) -> io::Result<Payload> {
    let mut at = Cursor {
        bytes: &bytes,
        at: 0,
    };
    if at.take(8)? != MAGIC {
        return Err(bad("not a Yappuccino payload"));
    }
    let version = at.string()?;
    let total = at.u64()?;
    Ok(Payload {
        version,
        total,
        stub_len,
        bytes,
    })
}

impl Payload {
    /// Writes every file under `dir`, telling `progress` the raw bytes done.
    pub fn unpack(&self, dir: &Path, mut progress: impl FnMut(u64)) -> io::Result<()> {
        let mut at = Cursor {
            bytes: &self.bytes,
            at: 0,
        };
        at.take(8)?;
        at.string()?;
        at.u64()?;
        let count = at.u32()?;
        let mut done = 0u64;
        for _ in 0..count {
            let name = at.string()?;
            let raw_len = at.u64()?;
            let deflated_len = at.u64()?;
            let deflated = at.take(deflated_len as usize)?;
            // The payload is data read from a file: nothing in it may land outside `dir`.
            let relative = Path::new(&name);
            if relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(bad(
                    "a file in the payload points outside the install folder",
                ));
            }
            let raw =
                miniz_oxide::inflate::decompress_to_vec_with_limit(deflated, raw_len as usize)
                    .map_err(|_| bad("a file in the payload is damaged"))?;
            if raw.len() as u64 != raw_len {
                return Err(bad("a file in the payload is damaged"));
            }
            let path = dir.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &raw)?;
            done += raw_len;
            progress(done);
        }
        Ok(())
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> io::Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| bad("payload cut short"))?;
        let part = &self.bytes[self.at..end];
        self.at = end;
        Ok(part)
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> io::Result<String> {
        let len = self.u32()? as usize;
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| bad("a name in the payload isn't UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_and_unpacks_through_a_setup_file() {
        let base =
            std::env::temp_dir().join(format!("yappuccino-setup-test-{}", std::process::id()));
        let source = base.join("source");
        fs::create_dir_all(source.join("licenses")).unwrap();
        fs::write(source.join("yappuccino.exe"), vec![7u8; 100_000]).unwrap();
        fs::write(source.join("licenses/LICENSE.txt"), "GPL").unwrap();
        fs::write(source.join("README.txt"), "only for the ZIP").unwrap();

        let payload = pack(&source, "0.3.0").unwrap();
        let setup = base.join("setup.exe");
        write_setup(b"STUB", &payload, &setup).unwrap();

        let read = read(&setup).unwrap().expect("carries a payload");
        assert_eq!(read.version, "0.3.0");
        assert_eq!(read.stub_len, 4);
        assert_eq!(read.total, 100_003);
        let target = base.join("target");
        let mut last = 0;
        read.unpack(&target, |done| last = done).unwrap();
        assert_eq!(last, 100_003);
        assert_eq!(
            fs::read(target.join("yappuccino.exe")).unwrap(),
            vec![7u8; 100_000]
        );
        assert_eq!(
            fs::read_to_string(target.join("licenses/LICENSE.txt")).unwrap(),
            "GPL"
        );
        assert!(!target.join("README.txt").exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn refuses_a_path_out_of_the_folder() {
        let mut payload = MAGIC.to_vec();
        put_str(&mut payload, "0");
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());
        put_str(&mut payload, "../evil.exe");
        let deflated = miniz_oxide::deflate::compress_to_vec(b"x", 1);
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&(deflated.len() as u64).to_le_bytes());
        payload.extend_from_slice(&deflated);
        let parsed = parse(payload, 0).unwrap();
        assert!(
            parsed
                .unpack(&std::env::temp_dir().join("yappuccino-never"), |_| ())
                .is_err()
        );
    }

    #[test]
    fn a_plain_program_carries_nothing() {
        let file = std::env::temp_dir().join(format!("yappuccino-plain-{}", std::process::id()));
        fs::write(&file, b"just a program, long enough to have a tail").unwrap();
        assert!(read(&file).unwrap().is_none());
        fs::remove_file(&file).unwrap();
    }
}
