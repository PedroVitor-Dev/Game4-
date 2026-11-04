//! Bounded header observations and explicitly weaker filename suggestions.
use crate::contracts::*;
use rusqlite::Connection;
use std::{collections::BTreeMap, path::Path};
pub const PREFIX_BYTES: usize = 64 * 1024;
pub const DETECTOR_VERSION: u32 = 1;
pub const FORMATS: &[FileFormat] = &[
    FileFormat::Unknown,
    FileFormat::Ines,
    FileFormat::N64Z64,
    FileFormat::N64V64,
    FileFormat::N64N64,
    FileFormat::SnesRom,
    FileFormat::GameBoyRom,
    FileFormat::GameBoyColorRom,
    FileFormat::GbaRom,
    FileFormat::NdsRom,
    FileFormat::N3dsRom,
    FileFormat::Cia,
    FileFormat::Nsp,
    FileFormat::Xci,
    FileFormat::MasterSystemRom,
    FileFormat::GameGearRom,
    FileFormat::MegaDriveRom,
    FileFormat::PcEngineRom,
    FileFormat::Adf,
    FileFormat::D64,
    FileFormat::Zip,
    FileFormat::SevenZip,
    FileFormat::Rar,
    FileFormat::Gzip,
    FileFormat::Chd,
    FileFormat::Iso9660,
    FileFormat::Cue,
    FileFormat::Pe,
    FileFormat::Elf,
    FileFormat::PsxExe,
    FileFormat::Xbe,
    FileFormat::DoomWad,
    FileFormat::RawBinary,
    FileFormat::RawRom,
];
pub const PLATFORMS: &[PlatformId] = &[
    PlatformId::Nes,
    PlatformId::Snes,
    PlatformId::GameBoy,
    PlatformId::GameBoyColor,
    PlatformId::GameBoyAdvance,
    PlatformId::NintendoDs,
    PlatformId::Nintendo3ds,
    PlatformId::NintendoSwitch,
    PlatformId::Nintendo64,
    PlatformId::MasterSystem,
    PlatformId::GameGear,
    PlatformId::MegaDrive,
    PlatformId::PcEngine,
    PlatformId::Amiga,
    PlatformId::Commodore64,
    PlatformId::Windows,
    PlatformId::Playstation,
    PlatformId::Xbox,
    PlatformId::Android,
];
pub const EVIDENCE: &[IdentificationEvidence] = &[
    IdentificationEvidence::Signature,
    IdentificationEvidence::Extension,
    IdentificationEvidence::Unknown,
];
type Result<T> = std::result::Result<T, IpcError>;
fn db_error(_: rusqlite::Error) -> IpcError {
    IpcError::new(ErrorCode::DatabaseUnavailable)
}
pub fn wire<T: serde::Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(v.into()))
        .map_err(|_| IpcError::new(ErrorCode::InvalidResponse))
}
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub format: FileFormat,
    pub platform: Option<PlatformId>,
}
fn platform_for(format: FileFormat) -> Option<PlatformId> {
    use FileFormat::*;
    match format {
        Ines => Some(PlatformId::Nes),
        N64Z64 | N64V64 | N64N64 => Some(PlatformId::Nintendo64),
        PsxExe => Some(PlatformId::Playstation),
        Xbe => Some(PlatformId::Xbox),
        _ => None,
    }
}
pub fn signature(prefix: &[u8]) -> Option<Signature> {
    use FileFormat::*;
    let format = if prefix.len() >= 16 && prefix.starts_with(b"NES\x1a") {
        Ines
    } else if prefix.len() >= 64 && prefix.starts_with(&[0x80, 0x37, 0x12, 0x40]) {
        N64Z64
    } else if prefix.len() >= 64 && prefix.starts_with(&[0x37, 0x80, 0x40, 0x12]) {
        N64V64
    } else if prefix.len() >= 64 && prefix.starts_with(&[0x40, 0x12, 0x37, 0x80]) {
        N64N64
    } else if prefix.len() >= 0x800 && prefix.starts_with(b"PS-X EXE") {
        PsxExe
    } else if prefix.len() >= 0x178 && prefix.starts_with(b"XBEH") {
        Xbe
    } else if (prefix.len() >= 30 && prefix.starts_with(b"PK\x03\x04"))
        || (prefix.len() >= 22 && prefix.starts_with(b"PK\x05\x06"))
    {
        Zip
    } else if prefix.len() >= 32 && prefix.starts_with(b"7z\xbc\xaf\x27\x1c") {
        SevenZip
    } else if prefix.starts_with(b"Rar!\x1a\x07\x00") || prefix.starts_with(b"Rar!\x1a\x07\x01\x00")
    {
        Rar
    } else if prefix.len() >= 10 && prefix.starts_with(b"\x1f\x8b\x08") {
        Gzip
    } else if prefix.len() >= 16 && prefix.starts_with(b"MComprHD") {
        Chd
    } else if prefix.len() >= 12 && (prefix.starts_with(b"IWAD") || prefix.starts_with(b"PWAD")) {
        DoomWad
    } else if prefix.len() >= 52
        && prefix.starts_with(b"\x7fELF")
        && matches!(prefix[4], 1 | 2)
        && matches!(prefix[5], 1 | 2)
        && prefix[6] == 1
        && (prefix[4] == 1 || prefix.len() >= 64)
    {
        Elf
    } else if pe_header(prefix) {
        Pe
    } else if prefix.len() >= 34816
        && matches!(prefix[32768], 1 | 2)
        && &prefix[32769..32774] == b"CD001"
        && prefix[32774] == 1
    {
        Iso9660
    } else {
        return None;
    };
    Some(Signature {
        format,
        platform: platform_for(format),
    })
}
fn pe_header(bytes: &[u8]) -> bool {
    if bytes.len() < 64 || !bytes.starts_with(b"MZ") {
        return false;
    }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    offset.checked_add(24).is_some_and(|end| end <= bytes.len())
        && bytes.get(offset..offset + 4) == Some(b"PE\0\0")
}
fn extension(path: &str) -> (FileFormat, Option<PlatformId>) {
    use FileFormat::*;
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "nes" => (Ines, Some(PlatformId::Nes)),
        "z64" => (N64Z64, Some(PlatformId::Nintendo64)),
        "v64" => (N64V64, Some(PlatformId::Nintendo64)),
        "n64" => (N64N64, Some(PlatformId::Nintendo64)),
        "sfc" | "smc" => (SnesRom, Some(PlatformId::Snes)),
        "gb" => (GameBoyRom, Some(PlatformId::GameBoy)),
        "gbc" => (GameBoyColorRom, Some(PlatformId::GameBoyColor)),
        "gba" => (GbaRom, Some(PlatformId::GameBoyAdvance)),
        "nds" => (NdsRom, Some(PlatformId::NintendoDs)),
        "3ds" => (N3dsRom, Some(PlatformId::Nintendo3ds)),
        "cia" => (Cia, Some(PlatformId::Nintendo3ds)),
        "nsp" => (Nsp, Some(PlatformId::NintendoSwitch)),
        "xci" => (Xci, Some(PlatformId::NintendoSwitch)),
        "sms" => (MasterSystemRom, Some(PlatformId::MasterSystem)),
        "gg" => (GameGearRom, Some(PlatformId::GameGear)),
        "gen" | "smd" => (MegaDriveRom, Some(PlatformId::MegaDrive)),
        "pce" | "sgx" => (PcEngineRom, Some(PlatformId::PcEngine)),
        "adf" => (Adf, Some(PlatformId::Amiga)),
        "d64" => (D64, Some(PlatformId::Commodore64)),
        "exe" | "dll" => (Pe, Some(PlatformId::Windows)),
        "elf" => (Elf, None),
        "xbe" => (Xbe, Some(PlatformId::Xbox)),
        "zip" | "jar" => (Zip, None),
        "apk" => (Zip, Some(PlatformId::Android)),
        "7z" => (SevenZip, None),
        "rar" => (Rar, None),
        "gz" | "tgz" => (Gzip, None),
        "chd" => (Chd, None),
        "iso" => (Iso9660, None),
        "cue" => (Cue, None),
        "wad" => (DoomWad, None),
        "bin" => (RawBinary, None),
        "rom" => (RawRom, None),
        _ => (Unknown, None),
    }
}
fn classify(
    path: String,
    size: i64,
    sha256: String,
    stored: Option<String>,
    version: Option<u32>,
) -> Result<LibraryFileInfo> {
    let hint = if size > 0 {
        extension(&path)
    } else {
        (FileFormat::Unknown, None)
    };
    let format = stored
        .as_deref()
        .map(decode::<FileFormat>)
        .transpose()?
        .filter(|f| *f != FileFormat::Unknown);
    let (format, format_evidence, platform, platform_evidence) = if let Some(format) = format {
        let signature_platform = platform_for(format);
        let platform = signature_platform.or_else(|| if hint.0 == format { hint.1 } else { None });
        (
            format,
            IdentificationEvidence::Signature,
            platform,
            if signature_platform.is_some() {
                IdentificationEvidence::Signature
            } else if platform.is_some() {
                IdentificationEvidence::Extension
            } else {
                IdentificationEvidence::Unknown
            },
        )
    } else {
        (
            hint.0,
            if hint.0 == FileFormat::Unknown {
                IdentificationEvidence::Unknown
            } else {
                IdentificationEvidence::Extension
            },
            hint.1,
            if hint.1.is_some() {
                IdentificationEvidence::Extension
            } else {
                IdentificationEvidence::Unknown
            },
        )
    };
    Ok(LibraryFileInfo {
        path,
        size_bytes: size.to_string(),
        sha256,
        format,
        format_evidence,
        platform,
        platform_evidence,
        detector_version: version,
    })
}
pub fn files(c: &Connection, id: i64, after: &str, limit: usize) -> Result<Vec<LibraryFileInfo>> {
    let mut stmt=c.prepare("SELECT cf.relative_path,f.size_bytes,f.sha256,fi.format_id,fi.detector_version FROM collection_item_files cf JOIN files f ON f.id=cf.file_id LEFT JOIN file_identifications fi ON fi.file_id=f.id WHERE cf.item_id=?1 AND cf.relative_path>?2 ORDER BY cf.relative_path LIMIT ?3").map_err(db_error)?;
    let raw = stmt
        .query_map(rusqlite::params![id, after, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<u32>>(4)?,
            ))
        })
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    raw.into_iter()
        .map(|(path, size, hash, format, version)| classify(path, size, hash, format, version))
        .collect()
}
pub fn summary(
    c: &Connection,
    id: i64,
) -> Result<(u32, String, Vec<FormatSummary>, Vec<PlatformSummary>)> {
    let records = files(c, id, "", 500_001)?;
    if records.len() > 500_000 {
        return Err(IpcError::new(ErrorCode::InvalidResponse));
    }
    let mut formats = BTreeMap::new();
    let mut platforms = BTreeMap::new();
    let mut bytes = 0u128;
    for r in &records {
        bytes += r
            .size_bytes
            .parse::<u128>()
            .map_err(|_| IpcError::new(ErrorCode::InvalidResponse))?;
        *formats.entry((r.format, r.format_evidence)).or_insert(0u32) += 1;
        if let Some(p) = r.platform {
            *platforms.entry((p, r.platform_evidence)).or_insert(0u32) += 1;
        }
    }
    Ok((
        records.len() as u32,
        bytes.to_string(),
        formats
            .into_iter()
            .map(|((format, evidence), file_count)| FormatSummary {
                format,
                evidence,
                file_count,
            })
            .collect(),
        platforms
            .into_iter()
            .map(|((platform, evidence), file_count)| PlatformSummary {
                platform,
                evidence,
                file_count,
            })
            .collect(),
    ))
}
pub fn item_files(
    c: &Connection,
    request: LibraryItemFilesRequest,
) -> Result<LibraryItemFilesPage> {
    let id = crate::manifests::item_id(&request.item_id)?;
    if !(1..=100).contains(&request.page.limit) {
        return Err(crate::library::invalid(ErrorField::Limit));
    }
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM collection_items WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    if !exists {
        return Err(IpcError::new(ErrorCode::NotFound));
    }
    let scope = format!("item-files-v1-{id}:");
    let after = match request.page.cursor {
        None => String::new(),
        Some(cursor) => {
            let hex = cursor
                .strip_prefix(&scope)
                .filter(|s| !s.is_empty() && s.len() % 2 == 0 && s.len() <= 131072)
                .ok_or_else(|| crate::library::invalid(ErrorField::Cursor))?;
            let bytes = hex
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|v| {
                    std::str::from_utf8(v)
                        .ok()
                        .and_then(|s| u8::from_str_radix(s, 16).ok())
                        .ok_or_else(|| crate::library::invalid(ErrorField::Cursor))
                })
                .collect::<Result<Vec<_>>>()?;
            String::from_utf8(bytes).map_err(|_| crate::library::invalid(ErrorField::Cursor))?
        }
    };
    let mut records = files(c, id, &after, usize::from(request.page.limit) + 1)?;
    let more = records.len() > usize::from(request.page.limit);
    records.truncate(usize::from(request.page.limit));
    let next_cursor = if more {
        records.last().map(|r| {
            format!(
                "{scope}{}",
                r.path
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        })
    } else {
        None
    };
    Ok(LibraryItemFilesPage {
        files: records,
        next_cursor,
    })
}
#[cfg(test)]
mod tests;
