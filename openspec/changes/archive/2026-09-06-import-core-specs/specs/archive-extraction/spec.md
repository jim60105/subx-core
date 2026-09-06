## ADDED Requirements

### Requirement: Zip Archive Extraction
The system SHALL extract the contents of `.zip` archive files using the
`zip` crate. Extraction SHALL preserve the archive-internal directory
structure within the temporary extraction directory. All extracted entries
SHALL have their paths validated against path-traversal attacks: any entry
whose resolved path falls outside the extraction root SHALL be skipped and
a warning SHALL be logged.

#### Scenario: Valid zip archive extracted
- **WHEN** `extract_archive` is called with a valid `.zip` file
- **THEN** all non-directory entries SHALL be extracted to the temp
  directory, preserving internal folder structure, and the function SHALL
  return the list of extracted file paths

#### Scenario: Zip entry with path traversal is rejected
- **WHEN** a `.zip` archive contains an entry with path `../../etc/passwd`
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Symlink entry in zip is rejected
- **WHEN** a `.zip` archive contains a symbolic link entry
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Empty zip archive
- **WHEN** `extract_archive` is called with a `.zip` that contains no
  file entries
- **THEN** the function SHALL return an empty path list and no error

### Requirement: RAR Archive Extraction
The system SHALL extract the contents of `.rar` archive files using the
`unrar` crate, which statically compiles the UnRAR C++ library via
`unrar_sys` — no runtime library dependency is required. Extraction SHALL
preserve internal directory structure. Path-traversal validation SHALL
apply identically to zip extraction, including rejection of entries
containing parent-directory components or absolute paths.
RAR support SHALL be gated behind a cargo feature flag `archive-rar`, the
real gate being `subx-core`'s own (`archive-rar = ["dep:unrar"]`;
`subx-cli`'s same-named feature is the pass-through that the release
workflow enables).
The feature is **enabled in release builds** so published binaries include
RAR support out of the box. It remains opt-in for development builds.

#### Scenario: Valid rar archive extracted
- **WHEN** `extract_archive` is called with a valid `.rar` file
- **THEN** all entries SHALL be extracted preserving folder structure and
  the function SHALL return the list of extracted file paths

#### Scenario: RAR feature disabled
- **WHEN** the binary is compiled without the `archive-rar` feature and a
  `.rar` file is supplied as input
- **THEN** the system SHALL log a warning indicating RAR support is not
  compiled in and SHALL skip the file

### Requirement: 7z Archive Extraction
The system SHALL extract the contents of `.7z` archive files using the
`sevenz-rust` crate. Extraction SHALL preserve the archive-internal directory
structure within the temporary extraction directory. All extracted entries
SHALL have their paths validated against path-traversal attacks: any entry
whose resolved path falls outside the extraction root SHALL be skipped and
a warning SHALL be logged. Symlinks SHALL be rejected. 7z support SHALL be
always-on (no feature flag) because `sevenz-rust` is pure Rust with no
native dependencies.

#### Scenario: Valid 7z archive extracted
- **WHEN** `extract_archive` is called with a valid `.7z` file containing
  `movie.srt` and `extras/bonus.ass`
- **THEN** both files SHALL be extracted to the temp directory preserving
  folder structure, and the function SHALL return the list of extracted
  file paths

#### Scenario: 7z entry with path traversal is rejected
- **WHEN** a `.7z` archive contains an entry with path `../../etc/passwd`
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Empty 7z archive
- **WHEN** `extract_archive` is called with a `.7z` that contains no
  file entries
- **THEN** the function SHALL return an empty path list and no error

#### Scenario: Password-protected 7z archive is skipped
- **WHEN** a password-protected `.7z` file is among the inputs
- **THEN** the system SHALL log a warning indicating password protection
  and SHALL continue processing other inputs

#### Scenario: 7z anti-item entries are skipped
- **WHEN** a `.7z` archive contains anti-item (deletion marker) or
  no-stream entries
- **THEN** those entries SHALL be skipped with a warning and SHALL NOT
  materialise as empty files on disk

### Requirement: Tar-Gzip Archive Extraction
The system SHALL extract the contents of `.tar.gz` and `.tgz` archive
files using the `tar` and `flate2` crates. Extraction SHALL preserve the
archive-internal directory structure. Path-traversal validation SHALL apply
identically to other formats. Only entries of type `Regular` and `Directory`
SHALL be extracted; all other entry types (symlinks, hard links, device
nodes, FIFOs) SHALL be skipped with a warning. Tar-gzip support SHALL be
always-on (no feature flag).

#### Scenario: Valid tar.gz archive extracted
- **WHEN** `extract_archive` is called with a valid `.tar.gz` file
  containing `sub1.srt` and `sub2.ass`
- **THEN** all regular file entries SHALL be extracted preserving folder
  structure and the function SHALL return the list of extracted file paths

#### Scenario: Tgz extension recognised
- **WHEN** a file named `subs.tgz` is supplied as input
- **THEN** the system SHALL treat it identically to a `.tar.gz` file and
  extract its contents

#### Scenario: Tar entry with symlink is rejected
- **WHEN** a `.tar.gz` archive contains a symbolic link entry
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Tar entry with hard link is rejected
- **WHEN** a `.tar.gz` archive contains a hard link entry
- **THEN** that entry SHALL be skipped, a warning SHALL be logged, and
  extraction SHALL continue for remaining entries

#### Scenario: Empty tar.gz archive
- **WHEN** `extract_archive` is called with a `.tar.gz` that contains no
  file entries
- **THEN** the function SHALL return an empty path list and no error

### Requirement: Format Detection by Extension
The system SHALL detect archive format by file extension (case-insensitive):
`.zip` → Zip format, `.rar` → RAR format, `.7z` → SevenZip format,
`.tar.gz` and `.tgz` → TarGz format. For `.tar.gz` detection, the system
SHALL check whether the full filename (case-insensitive) ends with `.tar.gz`
before falling through to single-extension matching. No magic-byte sniffing
SHALL be performed.

#### Scenario: Case-insensitive extension matching
- **WHEN** a file named `Subtitles.ZIP` is supplied as input
- **THEN** the system SHALL recognise it as a zip archive

#### Scenario: 7z extension recognised
- **WHEN** a file named `subtitles.7z` is supplied as input
- **THEN** the system SHALL recognise it as a 7z archive

#### Scenario: Tar.gz compound extension recognised
- **WHEN** a file named `subtitles.tar.gz` is supplied as input
- **THEN** the system SHALL recognise it as a tar-gzip archive

#### Scenario: Tgz extension recognised
- **WHEN** a file named `subtitles.tgz` is supplied as input
- **THEN** the system SHALL recognise it as a tar-gzip archive

#### Scenario: Case-insensitive tar.gz matching
- **WHEN** a file named `Subs.TAR.GZ` is supplied as input
- **THEN** the system SHALL recognise it as a tar-gzip archive

#### Scenario: Unknown extension is not treated as archive
- **WHEN** a file named `data.tar.bz2` is supplied as input
- **THEN** the system SHALL NOT attempt extraction and SHALL pass it
  through to normal file processing

#### Scenario: Plain gz file is not treated as tar.gz
- **WHEN** a file named `file.gz` is supplied as input (not `.tar.gz`)
- **THEN** the system SHALL NOT attempt extraction

### Requirement: Temporary Directory Lifecycle
The system SHALL create one `TempDir` per extracted archive. The temp
directory SHALL be automatically deleted when the owning
`CollectedFiles` struct is dropped. The temp directory SHALL be created
in the system's default temp location (`std::env::temp_dir()`).

#### Scenario: Temp dir cleaned up after command completes
- **WHEN** a command extracts an archive and then completes (success or
  error)
- **THEN** the temporary extraction directory SHALL no longer exist on
  disk

#### Scenario: Multiple archives each get their own temp dir
- **WHEN** two archive files are supplied as input
- **THEN** each SHALL be extracted to a separate `TempDir`

### Requirement: No Nested Archive Extraction
Archives found inside extracted archives SHALL NOT be extracted. They SHALL
be treated as regular files and subject to the normal extension filter
(which will skip them since `.zip`/`.rar`/`.7z`/`.tar.gz`/`.tgz` are not
media extensions).

#### Scenario: Archive inside archive is not extracted
- **WHEN** an outer `.zip` contains an inner `subs.7z` which in turn
  contains `movie.srt`
- **THEN** `movie.srt` SHALL NOT appear in the collected files; `subs.7z`
  SHALL be skipped by the extension filter

### Requirement: Direct-Input-Only Extraction
Only archive files directly specified via `subx-cli`'s `-i` flag (or
positional arguments) SHALL be eligible for extraction. Archives
discovered during directory traversal (when `-i` points to a directory
containing archive files) SHALL NOT be extracted — they SHALL be treated
as regular files and filtered by
extension.

#### Scenario: Archive in traversed directory is not extracted
- **GIVEN** the user runs `subx match -i /media/` and `/media/` contains
  `subs.zip` alongside `movie.mp4`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL NOT be extracted; it SHALL be skipped by the
  extension filter since `.zip` is not a media extension

#### Scenario: Directly specified archive is extracted
- **GIVEN** the user runs `subx match -i /media/subs.zip`
- **WHEN** `collect_files()` runs
- **THEN** `subs.zip` SHALL be extracted and its contents SHALL be
  processed

### Requirement: Error Handling for Corrupt or Protected Archives
The system SHALL NOT abort the command when an archive fails to extract.
Instead, it SHALL log a warning identifying the archive file and the
error reason, then continue processing remaining inputs.

#### Scenario: Corrupted archive is skipped
- **WHEN** a corrupted `.zip` file is among the inputs
- **THEN** the system SHALL log a warning for that file and SHALL
  continue processing other inputs

#### Scenario: Password-protected archive is skipped
- **WHEN** a password-protected `.rar` file is among the inputs
- **THEN** the system SHALL log a warning indicating password protection
  and SHALL continue processing other inputs

#### Scenario: Password-protected zip archive is skipped
- **WHEN** a password-protected `.zip` file is among the inputs
- **THEN** the system SHALL log a warning indicating password protection
  and SHALL continue processing other inputs

#### Scenario: Corrupted 7z archive is skipped
- **WHEN** a corrupted `.7z` file is among the inputs
- **THEN** the system SHALL log a warning for that file and SHALL
  continue processing other inputs

#### Scenario: Corrupted tar.gz archive is skipped
- **WHEN** a corrupted `.tar.gz` file is among the inputs
- **THEN** the system SHALL log a warning for that file and SHALL
  continue processing other inputs

### Requirement: Decompression Bomb Protection
The system SHALL enforce a maximum total expanded size (default 1 GiB) and
a maximum entry count (default 10,000 files) per archive. If either limit
is exceeded during extraction, the system SHALL abort extraction of that
archive, log a warning, and skip it.

#### Scenario: Archive exceeding size limit
- **WHEN** a `.7z` archive's extracted content would exceed 1 GiB
- **THEN** extraction SHALL stop, the partial temp dir SHALL be cleaned
  up, and the archive SHALL be skipped with a warning

#### Scenario: Archive exceeding entry count limit
- **WHEN** a `.tar.gz` archive contains more than 10,000 entries
- **THEN** extraction SHALL stop and the archive SHALL be skipped with a
  warning

#### Scenario: Zip archive exceeding size limit
- **WHEN** a `.zip` archive's extracted content would exceed 1 GiB
- **THEN** extraction SHALL stop, the partial temp dir SHALL be cleaned
  up, and the archive SHALL be skipped with a warning
