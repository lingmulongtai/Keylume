use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daw_note: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drum_note: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sysex_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_status: Option<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedDef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: String,
    pub group: String,
    pub pos: Point,
    pub size: Size,
    pub address: Address,
    pub verified: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceLayout {
    pub schema: u32,
    #[serde(default, rename = "geometryRevision")]
    pub geometry_revision: u32,
    pub model: String,
    pub canvas: Size,
    pub leds: Vec<LedDef>,
    pub decor: Value,
}
impl Default for DeviceLayout {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../resources/layout.json")).expect("bundled layout")
    }
}
impl DeviceLayout {
    pub fn upgrade_geometry(&mut self) -> bool {
        let old: Self = serde_json::from_str(include_str!("../../resources/layout-v1.json"))
            .expect("legacy bundled layout");
        if self.geometry_revision != 0
            || self.leds.len() != old.leds.len()
            || self.canvas.w != old.canvas.w
            || self.canvas.h != old.canvas.h
            || self.decor != old.decor
            || self.leds.iter().any(|led| {
                old.leds.iter().find(|v| v.id == led.id).is_none_or(|v| {
                    led.pos.x != v.pos.x
                        || led.pos.y != v.pos.y
                        || led.size.w != v.size.w
                        || led.size.h != v.size.h
                })
            })
        {
            return false;
        }
        let mut updated = Self::default();
        for led in &mut updated.leds {
            let previous = self.leds.iter().find(|v| v.id == led.id).unwrap();
            led.address = previous.address.clone();
            led.kind = previous.kind.clone();
            led.group = previous.group.clone();
            if previous.label.is_some() {
                led.label = previous.label.clone();
            }
            led.verified = previous.verified;
        }
        *self = updated;
        true
    }
    pub fn key_position(&self, note: u8) -> f32 {
        self.decor["keys"]
            .as_array()
            .and_then(|keys| {
                keys.iter()
                    .find(|k| k["note"].as_u64() == Some(note.clamp(36, 96) as u64))
            })
            .map(|k| {
                (k["x"].as_f64().unwrap_or(0.) + k["w"].as_f64().unwrap_or(0.) / 2.) as f32
                    / self.canvas.w
            })
            .unwrap_or(0.5)
    }
    pub fn keybed_y(&self) -> f32 {
        self.decor["keybed"]["y"].as_f64().unwrap_or(239.) as f32 / self.canvas.h
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || self.model != "launchkey-mk4-61" {
            return Err("未対応のレイアウトです".into());
        }
        if self.leds.is_empty()
            || self.leds.len() > 128
            || !self.canvas.w.is_finite()
            || self.canvas.w <= 0.
            || !self.canvas.h.is_finite()
            || self.canvas.h <= 0.
        {
            return Err("レイアウトのサイズが不正です".into());
        }
        for (key, count, fields) in [
            ("keys", 61, &["note", "x", "w"][..]),
            ("encoders", 8, &["x", "y", "r"][..]),
            ("faders", 9, &["x", "y", "h"][..]),
            ("wheels", 2, &["x", "y", "w", "h"][..]),
        ] {
            let values = self
                .decor
                .get(key)
                .and_then(Value::as_array)
                .ok_or("デバイス描画データがありません")?;
            if values.len() != count
                || values.iter().any(|v| {
                    fields.iter().any(|field| {
                        v.get(field)
                            .and_then(Value::as_f64)
                            .is_none_or(|n| !n.is_finite() || n.abs() > 10000.)
                    })
                })
            {
                return Err("デバイス描画データが不正です".into());
            }
        }
        if self.decor["keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k.get("black").and_then(Value::as_bool).is_none())
            || ["x", "y", "w", "h"].iter().any(|key| {
                self.decor["display"]
                    .get(key)
                    .and_then(Value::as_f64)
                    .is_none_or(|v| !v.is_finite() || v.abs() > 10000.)
            })
        {
            return Err("鍵盤 / 画面の描画データが不正です".into());
        }
        if let Some(bed) = self.decor.get("keybed") {
            if ["y", "h", "blackHeight"].iter().any(|key| {
                bed.get(key)
                    .and_then(Value::as_f64)
                    .is_none_or(|n| !n.is_finite() || !(0.0..=10000.0).contains(&n))
            }) {
                return Err("鍵盤のサイズが不正です".into());
            }
        }
        if let Some(controls) = self.decor.get("controls") {
            let controls = controls.as_array().ok_or("ボタンの描画データが不正です")?;
            if controls.len() > 64
                || controls.iter().any(|v| {
                    ["id", "label"].iter().any(|key| {
                        v.get(key)
                            .and_then(Value::as_str)
                            .is_none_or(|s| s.len() > 80)
                    }) || [("pos", "x"), ("pos", "y"), ("size", "w"), ("size", "h")]
                        .iter()
                        .any(|(group, key)| {
                            v[group][key]
                                .as_f64()
                                .is_none_or(|n| !n.is_finite() || !(0.0..=10000.0).contains(&n))
                        })
                })
            {
                return Err("ボタンの描画データが不正です".into());
            }
        }
        let mut ids = std::collections::HashSet::new();
        for led in &self.leds {
            if !ids.insert(&led.id)
                || led.id.is_empty()
                || led.id.len() > 80
                || led.label.as_ref().is_some_and(|label| label.len() > 80)
                || !["rgb", "mono", "none"].contains(&led.kind.as_str())
                || !["pads", "faderButtons", "buttons"].contains(&led.group.as_str())
            {
                return Err("LED の定義が不正です".into());
            }
            if (led.kind == "rgb" && led.address.sysex_id.is_none())
                || (led.kind == "mono" && led.address.cc.is_none())
                || (led.group == "pads"
                    && (led.address.daw_note.is_none() || led.address.drum_note.is_none()))
            {
                return Err("LED のアドレスがありません".into());
            }
            if !led.pos.x.is_finite()
                || !led.pos.y.is_finite()
                || !led.size.w.is_finite()
                || !led.size.h.is_finite()
                || led.size.w <= 0.
                || led.size.h <= 0.
            {
                return Err("LED の座標が不正です".into());
            }
            if [
                led.address.daw_note,
                led.address.drum_note,
                led.address.cc,
                led.address.sysex_id,
            ]
            .into_iter()
            .flatten()
            .any(|v| v > 127)
            {
                return Err("MIDI アドレスは 0–127 です".into());
            }
            if led
                .address
                .mono_status
                .is_some_and(|v| v != 0xb3 && v != 0x93)
            {
                return Err("単色ステータスは B3 / 93 です".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Zone {
    Named(String),
    Leds(Vec<String>),
}
impl Zone {
    pub fn contains(&self, led: &LedDef) -> bool {
        match self {
            Self::Leds(ids) => ids.contains(&led.id),
            Self::Named(z) => {
                z == "all"
                    || z == &led.group
                    || (z == "pads.top" && led.id.starts_with("pad.top"))
                    || (z == "pads.bottom" && led.id.starts_with("pad.bottom"))
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub effect: String,
    pub params: HashMap<String, Value>,
    pub zone: Zone,
    pub opacity: f32,
    pub blend: String,
    pub enabled: bool,
}
impl Layer {
    pub fn number(&self, k: &str, default: f32) -> f32 {
        self.params
            .get(k)
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }
    pub fn text<'a>(&'a self, k: &str, default: &'a str) -> &'a str {
        self.params
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or(default)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub brightness: f32,
    pub saturation: f32,
    pub temperature_k: f32,
    pub gamma: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettings {
    pub enabled: bool,
    pub widget: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_bits: Option<Vec<u8>>,
    pub show_on_preset_change: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub layers: Vec<Layer>,
    pub post: Post,
    pub display: Option<DisplaySettings>,
    #[serde(default)]
    pub builtin: bool,
}
pub const EFFECTS: &[&str] = &[
    "static",
    "paint",
    "gradient",
    "breathing",
    "spectrum_cycle",
    "wave",
    "ripple",
    "reactive",
    "starlight",
    "fire",
    "aurora",
    "audio_spectrum",
    "audio_pulse",
    "tempo_pulse",
    "note_map",
    "chord_color",
    "metronome",
    "hardware_fx",
];
impl Preset {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || !safe_id(&self.id)
            || self.name.trim().is_empty()
            || self.name.len() > 160
            || self.layers.len() > 32
        {
            return Err("プリセットの形式が不正です".into());
        }
        if !(0.0..=1.0).contains(&self.post.brightness)
            || !(0.0..=2.0).contains(&self.post.saturation)
            || !(0.1..=4.0).contains(&self.post.gamma)
            || !(1000.0..=12000.0).contains(&self.post.temperature_k)
        {
            return Err("ポスト処理の値が範囲外です".into());
        }
        let mut ids = std::collections::HashSet::new();
        for l in &self.layers {
            if !ids.insert(&l.id)
                || !EFFECTS.contains(&l.effect.as_str())
                || !(0.0..=1.0).contains(&l.opacity)
                || !["normal", "add", "multiply", "screen", "max"].contains(&l.blend.as_str())
            {
                return Err("レイヤーの形式が不正です".into());
            }
            match &l.zone {
                Zone::Named(z)
                    if ![
                        "all",
                        "pads",
                        "pads.top",
                        "pads.bottom",
                        "faderButtons",
                        "buttons",
                    ]
                    .contains(&z.as_str()) =>
                {
                    return Err("ゾーンが不正です".into())
                }
                Zone::Leds(ids) if ids.len() > 128 => return Err("ゾーンが大きすぎます".into()),
                _ => {}
            }
        }
        if self
            .display
            .as_ref()
            .and_then(|d| d.image_bits.as_ref())
            .is_some_and(|bits| bits.len() != 8192 || bits.iter().any(|&b| b > 1))
        {
            return Err("OLED は 128 × 64 の 1bit 画像です".into());
        }
        Ok(())
    }
}
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub fn builtin_presets() -> Vec<Preset> {
    serde_json::from_str(include_str!("../../resources/presets.json")).expect("bundled presets")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CoexistMode {
    LightingFirst,
    Handoff,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema: u32,
    pub active_preset: String,
    pub master_brightness: f32,
    pub fps: u32,
    pub coexist_mode: CoexistMode,
    pub mock: bool,
    pub keyboard_reactive: bool,
    pub daw_drum: bool,
    pub auto_repair: bool,
    pub fade_on_release: bool,
    pub daw_processes: Vec<String>,
    pub pads_port: String,
    pub controls_port: String,
    pub forwarding: bool,
    pub aftertouch: bool,
    pub pad_channel: u8,
    pub pad_notes: Vec<u8>,
    pub control_channel: Option<u8>,
    pub cc_map: HashMap<u8, u8>,
    pub shortcut_ccs: Vec<u8>,
    pub midi_log: bool,
    pub setup_complete: bool,
    pub idle_minutes: u32,
    pub idle_brightness: f32,
    pub night_enabled: bool,
    pub night_start: String,
    pub night_end: String,
    pub night_brightness: f32,
    pub manual_lock: bool,
    pub language: String,
    pub audio_device: String,
    pub check_for_updates: bool,
    pub include_prereleases: bool,
    pub piano: crate::piano::PianoSettings,
    pub controller: crate::controller::ControllerSettings,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            active_preset: "aurora".into(),
            master_brightness: 1.,
            fps: 30,
            coexist_mode: CoexistMode::LightingFirst,
            mock: true,
            keyboard_reactive: true,
            daw_drum: false,
            auto_repair: true,
            fade_on_release: true,
            daw_processes: vec![
                "Ableton Live*.exe",
                "FL64.exe",
                "Cubase*.exe",
                "Bitwig Studio.exe",
                "Studio One.exe",
                "reaper.exe",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            pads_port: "Keylume Pads".into(),
            controls_port: "Keylume Controls".into(),
            forwarding: true,
            aftertouch: true,
            pad_channel: 9,
            pad_notes: vec![
                40, 41, 42, 43, 48, 49, 50, 51, 36, 37, 38, 39, 44, 45, 46, 47,
            ],
            control_channel: None,
            cc_map: HashMap::new(),
            shortcut_ccs: vec![],
            midi_log: false,
            setup_complete: false,
            idle_minutes: 10,
            idle_brightness: 0.15,
            night_enabled: false,
            night_start: "23:00".into(),
            night_end: "07:00".into(),
            night_brightness: 0.2,
            manual_lock: false,
            language: "ja".into(),
            audio_device: String::new(),
            check_for_updates: true,
            include_prereleases: true,
            piano: Default::default(),
            controller: Default::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.piano.validate()?;
        self.controller.validate()?;
        if self.schema != 1
            || ![15, 30, 60].contains(&self.fps)
            || !(0.0..=1.0).contains(&self.master_brightness)
            || !(0.0..=1.0).contains(&self.idle_brightness)
            || !(0.0..=1.0).contains(&self.night_brightness)
            || self.pad_channel > 15
            || self.control_channel.is_some_and(|v| v > 15)
            || self.pad_notes.len() != 16
            || self.pad_notes.iter().any(|&v| v > 127)
            || self.cc_map.iter().any(|(&k, &v)| k > 127 || v > 127)
            || self.shortcut_ccs.iter().any(|&v| v > 127)
            || self.daw_processes.len() > 64
        {
            return Err("設定値が範囲外です".into());
        }
        if crate::profiles::minutes(&self.night_start).is_none()
            || crate::profiles::minutes(&self.night_end).is_none()
        {
            return Err("時間は HH:MM で指定してください".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<[String; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_minutes: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub r#match: ProfileMatch,
    pub preset_id: String,
    pub coexist_mode: CoexistMode,
    pub priority: i32,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if !safe_id(&self.id)
            || self.name.trim().is_empty()
            || !safe_id(&self.preset_id)
            || self
                .r#match
                .time_range
                .as_ref()
                .is_some_and(|r| r.iter().any(|t| crate::profiles::minutes(t).is_none()))
        {
            return Err("プロファイルの形式が不正です".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_data_is_valid() {
        DeviceLayout::default().validate().unwrap();
        for p in builtin_presets() {
            p.validate().unwrap();
        }
        Settings::default().validate().unwrap();
    }
    #[test]
    fn paths_cannot_escape_storage() {
        for s in ["../x", "C:\\x", "a/b", ".", ""] {
            assert!(!safe_id(s));
        }
    }
}
