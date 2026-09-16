use crate::model::{builtin_presets, DeviceLayout, Preset, Profile, Settings};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
pub struct Storage {
    pub root: PathBuf,
    pub notices: Vec<String>,
}
impl Storage {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        for dir in ["presets", "profiles", "layouts", "logs"] {
            fs::create_dir_all(root.join(dir)).map_err(|e| e.to_string())?;
        }
        Ok(Self {
            root,
            notices: vec![],
        })
    }
    pub fn load<T: DeserializeOwned + Serialize>(
        &mut self,
        path: &str,
        validate: impl Fn(&T) -> Result<(), String>,
    ) -> Option<T> {
        let file = self.root.join(path);
        if !file.exists() {
            return None;
        }
        let read = |p: &Path| -> Option<T> {
            let bytes = fs::read(p).ok()?;
            let v: T = serde_json::from_slice(&bytes).ok()?;
            validate(&v).ok()?;
            Some(v)
        };
        if let Some(v) = read(&file) {
            return Some(v);
        }
        if let Some(v) = read(&file.with_extension("json.bak")) {
            self.notices
                .push(format!("{path}: バックアップから復元しました"));
            if fs::rename(
                &file,
                file.with_extension(format!(
                    "corrupt-{}",
                    chrono::Local::now().timestamp_millis()
                )),
            )
            .is_ok()
            {
                let _ = self.save(path, &v);
            }
            return Some(v);
        }
        self.notices.push(format!(
            "{path}: 破損したファイルを保管し、既定値を使います"
        ));
        let _ = fs::copy(
            &file,
            file.with_extension(format!("corrupt-{}", chrono::Local::now().timestamp())),
        );
        None
    }
    pub fn settings(&mut self) -> Settings {
        self.load("settings.json", Settings::validate)
            .unwrap_or_default()
    }
    pub fn layout(&mut self) -> DeviceLayout {
        self.load("layouts/layout.json", DeviceLayout::validate)
            .unwrap_or_default()
    }
    pub fn presets(&mut self) -> Vec<Preset> {
        let mut presets = builtin_presets();
        for file in self.json_files("presets") {
            if let Some(p) = self.load::<Preset>(&file, Preset::validate) {
                if !presets.iter().any(|b| b.id == p.id) {
                    presets.push(p);
                }
            }
        }
        presets
    }
    pub fn profiles(&mut self) -> Vec<Profile> {
        self.json_files("profiles")
            .into_iter()
            .filter_map(|f| self.load(&f, Profile::validate))
            .collect()
    }
    fn json_files(&self, dir: &str) -> Vec<String> {
        fs::read_dir(self.root.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension()?.to_str()? == "json" {
                    Some(format!("{dir}/{}", p.file_name()?.to_str()?))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn save<T: Serialize>(&self, path: &str, value: &T) -> Result<(), String> {
        let file = self.root.join(path);
        let temp = file.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
        let mut f = fs::File::create(&temp).map_err(|e| e.to_string())?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        if file.exists() {
            fs::copy(&file, file.with_extension("json.bak")).map_err(|e| e.to_string())?;
        }
        fs::rename(temp, file).map_err(|e| e.to_string())
    }
    pub fn remove(&self, path: &str) -> Result<(), String> {
        let file = self.root.join(path);
        if file.exists() {
            fs::remove_file(&file).map_err(|e| e.to_string())?;
        }
        let _ = fs::remove_file(file.with_extension("json.bak"));
        Ok(())
    }
    pub fn log(&self, message: &str) {
        let today = chrono::Local::now();
        let name = format!("logs/keylume-{}.log", today.format("%Y%m%d"));
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(name))
        {
            let _ = writeln!(f, "{} {}", today.format("%H:%M:%S"), message);
        }
    }
    pub fn prune_logs(&self) {
        if let Ok(entries) = fs::read_dir(self.root.join("logs")) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("keylume-")
                    && entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age.as_secs() > 7 * 86400)
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}
pub fn import_preset(json: &str) -> Result<Preset, String> {
    if json.len() > 2_000_000 {
        return Err("プリセットは 2 MB 以下にしてください".into());
    }
    let mut v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    match v.get("schema").and_then(|v| v.as_u64()) {
        Some(0) => {
            v["schema"] = 1.into();
            if v.get("post").is_none() {
                v["post"] = serde_json::json!({"brightness":0.8,"saturation":1.,"temperatureK":6500.,"gamma":2.2});
            }
        }
        Some(1) => {}
        _ => return Err("このスキーマのプリセットは読み込めません".into()),
    }
    let mut preset: Preset = serde_json::from_value(v).map_err(|e| e.to_string())?;
    preset.builtin = false;
    preset.validate()?;
    Ok(preset)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_rejects_future_schema() {
        let mut v = serde_json::to_value(&builtin_presets()[0]).unwrap();
        v["schema"] = 0.into();
        v.as_object_mut().unwrap().remove("post");
        assert_eq!(import_preset(&v.to_string()).unwrap().schema, 1);
        v["schema"] = 999.into();
        assert!(import_preset(&v.to_string()).is_err());
    }
    #[test]
    fn corrupt_settings_recover_from_backup() {
        let root = std::env::temp_dir().join(format!(
            "keylume-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let mut s = Storage::new(root.clone()).unwrap();
        s.save("settings.json", &Settings::default()).unwrap();
        s.save("settings.json", &Settings::default()).unwrap();
        fs::write(root.join("settings.json"), b"broken").unwrap();
        assert_eq!(s.settings().fps, 30);
        assert_eq!(s.notices.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
