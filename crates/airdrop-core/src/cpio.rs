use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};

/// CPIO entry representing an archived file or directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpioEntry {
    pub name: String,
    pub mode: u32,
    pub mtime: u32,
    pub data: Vec<u8>,
}

/// Standard CPIO ODC magic bytes: "070707"
pub const CPIO_ODC_MAGIC: &[u8; 6] = b"070707";
/// Trailer record name signaling the end of a CPIO archive.
pub const CPIO_TRAILER: &str = "TRAILER!!!";

/// Builds a CPIO (odc) archive stream from a list of entries.
pub fn build_cpio_archive(entries: &[CpioEntry]) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    for entry in entries {
        write_cpio_entry(&mut buffer, entry)?;
    }

    // Write the mandatory trailer record
    let trailer_entry = CpioEntry {
        name: CPIO_TRAILER.to_string(),
        mode: 0,
        mtime: 0,
        data: Vec::new(),
    };
    write_cpio_entry(&mut buffer, &trailer_entry)?;

    Ok(buffer)
}

fn write_cpio_entry<W: Write>(writer: &mut W, entry: &CpioEntry) -> io::Result<()> {
    let name_bytes = entry.name.as_bytes();
    // name_len includes the null terminator
    let name_len = name_bytes.len() + 1;
    let file_size = entry.data.len();

    // Format fields in octal (ODC specification: 6 bytes each)
    let header = format!(
        "{:06}{:06o}{:06o}{:06o}{:06o}{:06o}{:06o}{:06o}{:011o}{:06o}{:011o}",
        "070707",          // magic
        0,                 // dev
        0,                 // ino
        entry.mode,        // mode
        0,                 // uid
        0,                 // gid
        1,                 // nlink
        0,                 // rdev
        entry.mtime,       // mtime (11 octal digits)
        name_len,          // namesize
        file_size,         // filesize (11 octal digits)
    );

    writer.write_all(header.as_bytes())?;
    writer.write_all(name_bytes)?;
    writer.write_all(&[0u8])?; // null terminator for name
    writer.write_all(&entry.data)?;

    Ok(())
}

/// Parses a CPIO (odc) archive from a byte slice into individual entries.
pub fn parse_cpio_archive(input: &[u8]) -> io::Result<Vec<CpioEntry>> {
    let mut cursor = Cursor::new(input);
    let mut entries = Vec::new();

    loop {
        let mut magic = [0u8; 6];
        if cursor.read_exact(&mut magic).is_err() {
            break; // EOF
        }

        if &magic != CPIO_ODC_MAGIC {
            // Check for newc magic "070701" or "070702"
            if &magic == b"070701" || &magic == b"070702" {
                return parse_newc_archive(input);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid CPIO magic: {:?}", String::from_utf8_lossy(&magic)),
            ));
        }

        let mut dev = [0u8; 6];
        let mut ino = [0u8; 6];
        let mut mode = [0u8; 6];
        let mut uid = [0u8; 6];
        let mut gid = [0u8; 6];
        let mut nlink = [0u8; 6];
        let mut rdev = [0u8; 6];
        let mut mtime = [0u8; 11];
        let mut namesize = [0u8; 6];
        let mut filesize = [0u8; 11];

        cursor.read_exact(&mut dev)?;
        cursor.read_exact(&mut ino)?;
        cursor.read_exact(&mut mode)?;
        cursor.read_exact(&mut uid)?;
        cursor.read_exact(&mut gid)?;
        cursor.read_exact(&mut nlink)?;
        cursor.read_exact(&mut rdev)?;
        cursor.read_exact(&mut mtime)?;
        cursor.read_exact(&mut namesize)?;
        cursor.read_exact(&mut filesize)?;

        let mode_val = parse_octal(&mode)?;
        let mtime_val = parse_octal(&mtime)?;
        let namesize_val = parse_octal(&namesize)? as usize;
        let filesize_val = parse_octal(&filesize)? as usize;

        let mut name_buf = vec![0u8; namesize_val];
        cursor.read_exact(&mut name_buf)?;

        // Strip null terminator
        if let Some(&0) = name_buf.last() {
            name_buf.pop();
        }
        let name_str = String::from_utf8_lossy(&name_buf).to_string();

        if name_str == CPIO_TRAILER {
            break; // Finished parsing archive
        }

        let mut data_buf = vec![0u8; filesize_val];
        cursor.read_exact(&mut data_buf)?;

        entries.push(CpioEntry {
            name: name_str,
            mode: mode_val,
            mtime: mtime_val,
            data: data_buf,
        });
    }

    Ok(entries)
}

fn parse_octal(bytes: &[u8]) -> io::Result<u32> {
    let s = std::str::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    u32::from_str_radix(s.trim(), 8).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn parse_hex(bytes: &[u8]) -> io::Result<u32> {
    let s = std::str::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    u32::from_str_radix(s.trim(), 16).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Fallback parser for standard SVR4 / newc CPIO format.
fn parse_newc_archive(input: &[u8]) -> io::Result<Vec<CpioEntry>> {
    let mut cursor = Cursor::new(input);
    let mut entries = Vec::new();

    loop {
        let mut magic = [0u8; 6];
        if cursor.read_exact(&mut magic).is_err() {
            break;
        }

        if &magic != b"070701" && &magic != b"070702" {
            break;
        }

        let mut header = [0u8; 104]; // 13 fields * 8 hex bytes
        cursor.read_exact(&mut header)?;

        let mode_val = parse_hex(&header[8..16])?;
        let mtime_val = parse_hex(&header[32..40])?;
        let filesize_val = parse_hex(&header[40..48])? as usize;
        let namesize_val = parse_hex(&header[72..80])? as usize;

        let mut name_buf = vec![0u8; namesize_val];
        cursor.read_exact(&mut name_buf)?;

        // Pad name to 4 bytes boundary
        let name_padding = (4 - ((110 + namesize_val) % 4)) % 4;
        let mut pad = vec![0u8; name_padding];
        cursor.read_exact(&mut pad)?;

        if let Some(&0) = name_buf.last() {
            name_buf.pop();
        }
        let name_str = String::from_utf8_lossy(&name_buf).to_string();

        if name_str == CPIO_TRAILER {
            break;
        }

        let mut data_buf = vec![0u8; filesize_val];
        cursor.read_exact(&mut data_buf)?;

        // Pad data to 4 bytes boundary
        let data_padding = (4 - (filesize_val % 4)) % 4;
        let mut dpad = vec![0u8; data_padding];
        cursor.read_exact(&mut dpad)?;

        entries.push(CpioEntry {
            name: name_str,
            mode: mode_val,
            mtime: mtime_val,
            data: data_buf,
        });
    }

    Ok(entries)
}

/// Decompresses a DVZip (application/x-dvzip) payload.
/// Attempts Zlib decoding, then Gzip decoding, then raw Deflate decoding.
pub fn decompress_dvzip(input: &[u8]) -> io::Result<Vec<u8>> {
    // 1. Try Zlib
    let mut zlib_decoder = ZlibDecoder::new(input);
    let mut output = Vec::new();
    if zlib_decoder.read_to_end(&mut output).is_ok() && !output.is_empty() {
        return Ok(output);
    }

    // 2. Try Gzip
    output.clear();
    let mut gz_decoder = GzDecoder::new(input);
    if gz_decoder.read_to_end(&mut output).is_ok() && !output.is_empty() {
        return Ok(output);
    }

    // 3. Try Raw Deflate
    output.clear();
    let mut deflate_decoder = DeflateDecoder::new(input);
    if deflate_decoder.read_to_end(&mut output).is_ok() && !output.is_empty() {
        return Ok(output);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to decompress DVZip payload: not valid zlib, gzip, or deflate stream",
    ))
}

/// Compresses raw CPIO data into DVZip (application/x-dvzip) using zlib compression.
pub fn compress_dvzip(input: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input)?;
    encoder.finish()
}

/// Automatically detects and parses an AirDrop upload payload.
/// Handles raw CPIO (odc or newc) as well as compressed DVZip (zlib/gzip/deflate).
pub fn parse_archive_auto(input: &[u8]) -> io::Result<Vec<CpioEntry>> {
    if input.starts_with(CPIO_ODC_MAGIC) || input.starts_with(b"070701") || input.starts_with(b"070702") {
        return parse_cpio_archive(input);
    }

    // If not starting with standard CPIO magic, attempt DVZip decompression
    match decompress_dvzip(input) {
        Ok(decompressed) => parse_cpio_archive(&decompressed),
        Err(_) => {
            // Fall back to attempting direct parse
            parse_cpio_archive(input)
        }
    }
}

/// Extracts parsed CPIO entries onto the filesystem under a target base directory.
pub fn extract_cpio_to_directory(entries: &[CpioEntry], target_dir: &Path) -> io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(target_dir)?;
    let target_canonical = target_dir.canonicalize()?;
    let mut written_files = Vec::new();

    for entry in entries {
        // Prevent null byte injection
        if entry.name.contains('\0') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Filename contains null byte"));
        }

        // Prevent path traversal: take only the safe basename or relative parts without parent navigation
        let path = Path::new(&entry.name);
        let safe_name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };

        let file_path = target_canonical.join(safe_name);

        // Verify the path is strictly contained within target_canonical
        if !file_path.starts_with(&target_canonical) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Path traversal attempt detected"));
        }

        std::fs::write(&file_path, &entry.data)?;
        written_files.push(file_path);
    }

    Ok(written_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpio_roundtrip() {
        let entries = vec![
            CpioEntry {
                name: "test.txt".to_string(),
                mode: 0o100644,
                mtime: 1729440000,
                data: b"Hello AirDrop from Android!".to_vec(),
            },
            CpioEntry {
                name: "images/sample.jpg".to_string(),
                mode: 0o100644,
                mtime: 1729440000,
                data: vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10],
            },
        ];

        let archive_bytes = build_cpio_archive(&entries).expect("build cpio failed");
        assert!(archive_bytes.len() > 0);

        let parsed = parse_cpio_archive(&archive_bytes).expect("parse cpio failed");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "test.txt");
        assert_eq!(parsed[0].data, b"Hello AirDrop from Android!");
        assert_eq!(parsed[1].name, "images/sample.jpg");
        assert_eq!(parsed[1].data, vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]);
    }

    #[test]
    fn test_extract_path_traversal_containment() {
        let temp_dir = std::env::temp_dir().join(format!("airdrop_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let malicious_entries = vec![
            CpioEntry {
                name: "../../../../evil.sh".to_string(),
                mode: 0o100755,
                mtime: 0,
                data: b"echo pwned".to_vec(),
            },
            CpioEntry {
                name: "normal.txt".to_string(),
                mode: 0o100644,
                mtime: 0,
                data: b"safe content".to_vec(),
            },
        ];

        let extracted = extract_cpio_to_directory(&malicious_entries, &temp_dir).expect("extraction failed");
        // Verify evil.sh was constrained to basename inside temp_dir and did NOT escape
        for path in &extracted {
            assert!(path.starts_with(&temp_dir.canonicalize().unwrap()));
            assert!(!path.to_str().unwrap().contains(".."));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extract_null_byte_rejection() {
        let temp_dir = std::env::temp_dir().join(format!("airdrop_null_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let null_entries = vec![
            CpioEntry {
                name: "evil.sh\0.txt".to_string(),
                mode: 0o100644,
                mtime: 0,
                data: b"bad".to_vec(),
            },
        ];

        let res = extract_cpio_to_directory(&null_entries, &temp_dir);
        assert!(res.is_err(), "Null byte in filename must return Err");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dvzip_compression_decompression_roundtrip() {
        let original_data = b"Universal AirDrop high-volume data stream payload for Apple and Android interoperability";
        let compressed = compress_dvzip(original_data).expect("compress_dvzip failed");
        assert!(!compressed.is_empty());

        let decompressed = decompress_dvzip(&compressed).expect("decompress_dvzip failed");
        assert_eq!(&decompressed, original_data);
    }

    #[test]
    fn test_parse_archive_auto_with_dvzip() {
        let entries = vec![
            CpioEntry {
                name: "compressed_photo.heic".to_string(),
                mode: 0o100644,
                mtime: 1729441234,
                data: vec![0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x68, 0x65, 0x69, 0x63],
            },
        ];

        let cpio_bytes = build_cpio_archive(&entries).expect("build cpio failed");
        let dvzip_bytes = compress_dvzip(&cpio_bytes).expect("compress dvzip failed");

        // Verify parse_archive_auto transparently extracts from the DVZip payload
        let parsed = parse_archive_auto(&dvzip_bytes).expect("parse_archive_auto failed");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "compressed_photo.heic");
        assert_eq!(parsed[0].data, vec![0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x68, 0x65, 0x69, 0x63]);
    }
}
