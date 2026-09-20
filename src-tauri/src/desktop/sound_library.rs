use super::runtime::Core;
use crate::{
    sound_library::{self as library, SoundEntry, SoundId, PACK_FILE, PACK_SHA, PACK_SIZE},
    storage::Storage,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const DOWNLOAD: &str = "https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/684543d5e5efaef08d02be50dcda8d552478fa60/GeneralUser-GS.sf2";
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub active: bool,
    pub received: u64,
    pub total: u64,
    pub error: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryView {
    #[serde(flatten)]
    pub entry: SoundEntry,
    pub installed: bool,
}
pub struct Library {
    root: PathBuf,
    imported: Mutex<Vec<SoundEntry>>,
    busy: AtomicBool,
    cancel: AtomicBool,
    progress: Mutex<Progress>,
}
impl Library {
    pub fn new(root: PathBuf) -> Arc<Self> {
        let _ = fs::create_dir_all(&root);
        let imported = fs::metadata(root.join("imports.json"))
            .ok()
            .filter(|m| m.len() < 8 * 1024 * 1024)
            .and_then(|_| fs::read(root.join("imports.json")).ok())
            .filter(|b| b.len() < 8 * 1024 * 1024)
            .and_then(|b| serde_json::from_slice::<Vec<SoundEntry>>(&b).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|e| {
                e.id.starts_with("user:") && e.name.len() <= 400 && SoundId::parse(&e.id).is_ok()
            })
            .take(16384)
            .collect();
        Arc::new(Self {
            root,
            imported: Mutex::new(imported),
            busy: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            progress: Mutex::new(Progress::default()),
        })
    }
    pub fn entries(&self) -> Vec<EntryView> {
        let mut catalog = library::catalog();
        catalog.extend(self.imported.lock().unwrap().clone());
        let mut installed = std::collections::HashMap::new();
        catalog
            .into_iter()
            .map(|entry| {
                let id = SoundId::parse(&entry.id).expect("validated catalog");
                let ready = *installed
                    .entry(id.key.clone())
                    .or_insert_with(|| id.path(&self.root).is_none_or(|p| p.is_file()));
                EntryView {
                    entry,
                    installed: ready,
                }
            })
            .collect()
    }
    pub fn load(&self, id: &str) -> Result<Arc<rustysynth::SoundFont>, String> {
        let sound = SoundId::parse(id)?;
        let Some(path) = sound.path(&self.root) else {
            return crate::piano::sound_font_for(id);
        };
        let (bytes, font) = library::read_font(&path)?;
        if sound.key == "generaluser" && library::digest(&bytes) != PACK_SHA {
            return Err("追加音源の整合性を確認できません。音源画面から再取得してください".into());
        }
        if sound.key.starts_with("user-")
            && sound.key != format!("user-{}", library::digest(&bytes))
        {
            return Err("持ち込み音源が変更されています。もう一度インポートしてください".into());
        }
        if !font.get_presets().iter().any(|p| {
            p.get_bank_number() == sound.bank as i32 && p.get_patch_number() == sound.program as i32
        }) {
            return Err("音源にこの音色がありません".into());
        }
        Ok(font)
    }
    fn begin(&self) -> Result<(), String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "音源を準備中です")?;
        self.cancel.store(false, Ordering::Release);
        *self.progress.lock().unwrap() = Progress {
            active: true,
            total: PACK_SIZE,
            ..Default::default()
        };
        Ok(())
    }
    fn finish(&self, result: &Result<(), String>) {
        let mut p = self.progress.lock().unwrap();
        p.active = false;
        p.error = result.as_ref().err().cloned().unwrap_or_default();
        self.busy.store(false, Ordering::Release);
    }
    fn download(&self) -> Result<(), String> {
        let final_path = self.root.join(PACK_FILE);
        if fs::metadata(&final_path).is_ok_and(|m| m.len() == PACK_SIZE)
            && fs::read(&final_path).is_ok_and(|b| library::digest(&b) == PACK_SHA)
        {
            return Ok(());
        }
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(DOWNLOAD)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("音源を取得できません: {e}"))?;
        if response.content_length().is_some_and(|n| n != PACK_SIZE) {
            return Err("音源サイズが一致しません".into());
        }
        let part = self.root.join("generaluser.download");
        let result = (|| {
            let mut file = fs::File::create(&part).map_err(|e| e.to_string())?;
            let mut bytes = Vec::with_capacity(PACK_SIZE as usize);
            let mut block = [0u8; 64 * 1024];
            let started = Instant::now();
            loop {
                if self.cancel.load(Ordering::Acquire) {
                    return Err("ダウンロードをキャンセルしました".into());
                }
                if started.elapsed() > Duration::from_secs(180) {
                    return Err("音源取得がタイムアウトしました".into());
                }
                let n = response.read(&mut block).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                if bytes.len() + n > PACK_SIZE as usize {
                    return Err("音源サイズが大きすぎます".into());
                }
                file.write_all(&block[..n]).map_err(|e| e.to_string())?;
                bytes.extend_from_slice(&block[..n]);
                self.progress.lock().unwrap().received = bytes.len() as u64;
            }
            if bytes.len() as u64 != PACK_SIZE || library::digest(&bytes) != PACK_SHA {
                return Err("音源のハッシュが一致しません。再試行してください".into());
            }
            library::font(&bytes)?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            // Only the verified complete bank becomes visible to the audio manager.
            fs::rename(&part, &final_path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(part);
        }
        result
    }
    fn import(&self, path: PathBuf) -> Result<Vec<String>, String> {
        let (bytes, font) = library::read_font(&path)?;
        let entries = library::entries_for_import(&bytes, &font);
        if entries.is_empty() {
            return Err("対応する音色がありません".into());
        }
        let id = SoundId::parse(&entries[0].id)?;
        let target = id.path(&self.root).ok_or("保存先が不正です")?;
        let mut imported = self.imported.lock().unwrap();
        let mut next = imported.clone();
        next.retain(|e| !entries.iter().any(|new| new.id == e.id));
        next.extend(entries.clone());
        if next.len() > 16384 {
            return Err("音色ライブラリが上限に達しました".into());
        }
        let part = self.root.join("import.download");
        let mut file = fs::File::create(&part).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&part, &target).map_err(|e| e.to_string())?;
        Storage::new(self.root.clone())?.save("imports.json", &next)?;
        *imported = next;
        Ok(entries.into_iter().map(|e| e.id).collect())
    }
}
#[tauri::command]
pub async fn library_command(
    name: String,
    args: Value,
    core: tauri::State<'_, Arc<Core>>,
) -> Result<Value, String> {
    let library = core.library.clone();
    match name.as_str() {
        "state" => {
            return Ok(
                json!({"entries":library.entries(),"progress":library.progress.lock().unwrap().clone()}),
            )
        }
        "cancel" => {
            library.cancel.store(true, Ordering::Release);
            return Ok(Value::Null);
        }
        "download" | "import" => {}
        _ => return Err("未対応の音源操作です".into()),
    }
    let path = if name == "import" {
        Some(PathBuf::from(
            args["path"]
                .as_str()
                .filter(|p| p.len() < 32768)
                .ok_or("音源ファイルを選んでください")?,
        ))
    } else {
        None
    };
    library.begin()?;
    let worker = library.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        if let Some(path) = path {
            worker.import(path).map(|ids| json!(ids))
        } else {
            worker.download().map(|_| Value::Null)
        }
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|v| v);
    library.finish(&result.as_ref().map(|_| ()).map_err(Clone::clone));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_are_owned_verified_repairable_and_persisted() {
        let root = std::env::temp_dir().join(format!(
            "keylume-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let library = Library::new(root.clone());
        let source = root.join("original.sf2");
        fs::write(
            &source,
            include_bytes!("../../../resources/piano/fm-piano.sf2"),
        )
        .unwrap();
        let ids = library.import(source.clone()).unwrap();
        assert!(!ids.is_empty());
        let target = SoundId::parse(&ids[0]).unwrap().path(&root).unwrap();
        assert_ne!(source, target);
        assert!(library.load(&ids[0]).is_ok());
        fs::write(&target, b"corrupted").unwrap();
        assert!(library.load(&ids[0]).is_err());
        assert_eq!(library.import(source.clone()).unwrap(), ids);
        fs::remove_file(source).unwrap();
        assert!(library.load(&ids[0]).is_ok());
        let reloaded = Library::new(root.clone());
        assert_eq!(
            reloaded
                .entries()
                .iter()
                .filter(|e| e.entry.id.starts_with("user:"))
                .count(),
            ids.len()
        );
        assert!(reloaded.load(&ids[0]).is_ok());
        assert!(library.begin().is_ok());
        assert!(library.begin().is_err());
        library.finish(&Err("cancelled".into()));
        assert!(library.begin().is_ok());
        library.finish(&Ok(()));
        fs::remove_dir_all(root).unwrap();
    }
}
