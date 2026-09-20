use rustysynth::SoundFont;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const PACK_SIZE: u64 = 32_319_396;
pub const PACK_SHA: &str = "9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe";
pub const PACK_FILE: &str = "GeneralUser-GS-2.0.3.sf2";
pub const MAX_FONT: usize = 256 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub bank: u16,
    pub program: u8,
}
pub fn catalog() -> Vec<SoundEntry> {
    let mut entries = vec![];
    for (id, name) in [
        ("upright", "Upright Piano"),
        ("bright", "Bright Upright"),
        ("fm-piano", "FM Electric"),
        ("honky-tonk", "Honky-tonk"),
    ] {
        entries.push(SoundEntry {
            id: id.into(),
            name: name.into(),
            category: "Piano".into(),
            bank: 0,
            program: 0,
        });
    }
    entries.extend(
        serde_json::from_str::<Vec<SoundEntry>>(include_str!("../../resources/sound-library.json"))
            .expect("checked factory catalog"),
    );
    entries
}
#[derive(Clone, Debug, PartialEq)]
pub struct SoundId {
    pub key: String,
    pub bank: u16,
    pub program: u8,
}
impl SoundId {
    pub fn parse(id: &str) -> Result<Self, String> {
        if ["upright", "bright", "fm-piano", "honky-tonk"].contains(&id) {
            return Ok(Self {
                key: id.into(),
                bank: 0,
                program: 0,
            });
        }
        let p: Vec<_> = id.split(':').collect();
        let (key, b, p) = match p.as_slice() {
            ["generaluser", b, p] => ("generaluser".into(), *b, *p),
            ["user", hash, b, p]
                if hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()) =>
            {
                (format!("user-{hash}"), *b, *p)
            }
            _ => return Err("音源IDが不正です".into()),
        };
        let bank = b.parse::<u16>().map_err(|_| "音源バンクが不正です")?;
        let program = p.parse::<u8>().map_err(|_| "音色番号が不正です")?;
        if bank > 16383 || program > 127 {
            return Err("音色が範囲外です".into());
        }
        Ok(Self { key, bank, program })
    }
    pub fn path(&self, root: &Path) -> Option<PathBuf> {
        match self.key.as_str() {
            "generaluser" => Some(root.join(PACK_FILE)),
            key if key.starts_with("user-") => Some(root.join(format!("{key}.sf2"))),
            _ => None,
        }
    }
}
pub fn font(bytes: &[u8]) -> Result<Arc<SoundFont>, String> {
    if bytes.len() < 12
        || bytes.len() > MAX_FONT
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"sfbk"
    {
        return Err("SoundFont 2（.sf2）を選んでください。最大256MBです".into());
    }
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if size.checked_add(8) != Some(bytes.len()) {
        return Err("SoundFontのサイズ情報が壊れています".into());
    }
    fn chunks(bytes: &[u8], depth: usize) -> Result<(), String> {
        if depth > 4 {
            return Err("SoundFontリストの階層が深すぎます".into());
        }
        let mut i = 0;
        while i < bytes.len() {
            if bytes.len() - i < 8 {
                return Err("SoundFontのチャンクが不完全です".into());
            }
            let n = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
            let end = i
                .checked_add(8)
                .and_then(|i| i.checked_add(n))
                .filter(|end| *end <= bytes.len())
                .ok_or("SoundFontのチャンクが範囲外です")?;
            if &bytes[i..i + 4] == b"LIST" {
                if n < 4 {
                    return Err("空のSoundFontリストです".into());
                }
                chunks(&bytes[i + 12..end], depth + 1)?;
            }
            i = end + (n % 2);
            if i > bytes.len() {
                return Err("SoundFontの境界が不正です".into());
            }
        }
        Ok(())
    }
    chunks(&bytes[12..], 0)?;
    let result = std::panic::catch_unwind(|| SoundFont::new(&mut Cursor::new(bytes)))
        .map_err(|_| "SoundFontを解析できません")?
        .map_err(|e| e.to_string())?;
    if result.get_presets().is_empty() || result.get_presets().len() > 1024 {
        return Err("音色数は1〜1024までです".into());
    }
    Ok(Arc::new(result))
}
pub fn read_font(path: &Path) -> Result<(Vec<u8>, Arc<SoundFont>), String> {
    use std::io::Read;
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    if f.metadata().map_err(|e| e.to_string())?.len() > MAX_FONT as u64 {
        return Err("SoundFontは最大256MBです".into());
    }
    let mut bytes = vec![];
    f.take(MAX_FONT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let font = font(&bytes)?;
    Ok((bytes, font))
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn entries_for_import(bytes: &[u8], font: &SoundFont) -> Vec<SoundEntry> {
    let hash = digest(bytes);
    font.get_presets()
        .iter()
        .filter(|p| {
            (0..=16383).contains(&p.get_bank_number()) && (0..=127).contains(&p.get_patch_number())
        })
        .map(|p| SoundEntry {
            id: format!(
                "user:{hash}:{}:{}",
                p.get_bank_number(),
                p.get_patch_number()
            ),
            name: p.get_name().trim().chars().take(100).collect(),
            category: "Imported".into(),
            bank: p.get_bank_number() as u16,
            program: p.get_patch_number() as u8,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_cannot_escape_the_library_and_catalog_is_unique() {
        for id in [
            "../font.sf2",
            "user:../../x:0:0",
            "generaluser:0:128",
            "generaluser:65535:0",
        ] {
            assert!(SoundId::parse(id).is_err());
        }
        let c = catalog();
        let ids: std::collections::HashSet<_> = c.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), c.len());
        assert!(c.len() > 250);
        for e in c {
            let s = SoundId::parse(&e.id).unwrap();
            assert_eq!((s.bank, s.program), (e.bank, e.program));
        }
    }
    #[test]
    fn imports_validate_structure_and_keep_every_patch() {
        let bytes = include_bytes!("../../resources/piano/upright.sf2");
        let parsed = font(bytes).unwrap();
        let entries = entries_for_import(bytes, &parsed);
        assert!(!entries.is_empty());
        assert!(entries[0].id.starts_with("user:"));
        assert!(font(&bytes[..100]).is_err());
        let mut invalid = bytes.to_vec();
        invalid[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(font(&invalid).is_err());
    }
}
