//! ZIP archive extraction.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use log::{debug, warn};

use super::common::ExtractionLimits;

/// Reports an entry that extraction refused, without aborting the archive.
fn warn_skipped(reason: &str, archive: &Path, name: &str) {
    warn!("Skipping {reason} entry in {}: {name}", archive.display());
}

/// Extracts a ZIP archive to `dest_dir`.
///
/// Validates each entry against path traversal, rejects symlinks and
/// hardlinks, and enforces decompression bomb limits.
pub(super) fn extract_zip(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to open zip archive {}: {e}", archive_path.display()),
        )
    })?;

    let dest_canonical = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());
    let mut extracted_paths = Vec::new();
    let mut limits = ExtractionLimits::new(archive_path);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read zip entry {i}: {e}"),
            )
        })?;

        if entry.is_dir() {
            continue;
        }

        // zip 9 decodes the stored name and reports failure as an error. The
        // log line must never turn one malformed header into an aborted
        // extraction, so it reports the raw bytes instead.
        let entry_name = String::from_utf8_lossy(entry.name_raw());

        if entry.is_symlink() {
            warn_skipped("symlink", archive_path, &entry_name);
            continue;
        }

        limits.check_entry(entry.size())?;

        // Path traversal validation (zip-specific enclosed_name)
        let entry_path = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                warn_skipped("path-traversal", archive_path, &entry_name);
                continue;
            }
        };

        let target_path = dest_dir.join(&entry_path);
        if let Ok(canonical) = target_path.canonicalize() {
            if !canonical.starts_with(&dest_canonical) {
                warn_skipped("path-traversal", archive_path, &entry_name);
                continue;
            }
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = fs::File::create(&target_path)?;
        io::copy(&mut entry, &mut outfile)?;

        debug!("Extracted: {}", target_path.display());
        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_zip(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let zip_path = dir.join("test.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
        zip_path
    }

    #[test]
    fn test_extract_zip_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = create_test_zip(
            tmp.path(),
            &[
                ("subtitle.srt", b"1\n00:00:01,000 --> 00:00:02,000\nHello\n"),
                ("subdir/another.ass", b"[Script Info]\nTitle: Test\n"),
            ],
        );

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(result.len(), 2);
        assert!(dest.join("subtitle.srt").exists());
        assert!(dest.join("subdir/another.ass").exists());
    }

    #[test]
    fn test_extract_zip_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = create_test_zip(tmp.path(), &[]);
        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_zip_path_traversal_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("malicious.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        writer.start_file("../../etc/passwd", options).unwrap();
        writer
            .write_all(b"root:x:0:0:root:/root:/bin/bash\n")
            .unwrap();
        writer.start_file("valid.srt", options).unwrap();
        writer.write_all(b"valid content").unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_zip_symlink_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("symlink.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // A symlink entry must never be materialized: the target is outside
        // the extraction directory and the entry itself is not a subtitle.
        writer
            .add_symlink("link.srt", "../../outside.srt", options)
            .unwrap();
        writer.start_file("valid.srt", options).unwrap();
        writer.write_all(b"valid content").unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(result, vec![dest.join("valid.srt")]);
        assert!(!dest.join("link.srt").exists());
    }

    #[test]
    fn test_extract_zip_entry_count_exceeded() {
        use super::super::common::MAX_ENTRY_COUNT;

        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("many_entries.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for i in 0..=MAX_ENTRY_COUNT {
            writer.start_file(format!("file_{i}.txt"), options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("entry count"));
    }
}
