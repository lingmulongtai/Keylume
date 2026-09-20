use crate::model::DeviceLayout;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Performance,
    Desktop,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Binding {
    pub action: String,
    pub value: String,
}
impl Default for Binding {
    fn default() -> Self {
        Self {
            action: "none".into(),
            value: String::new(),
        }
    }
}
impl Binding {
    pub fn new(action: &str, value: &str) -> Self {
        Self {
            action: action.into(),
            value: value.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControllerSettings {
    pub enabled: bool,
    pub mode: Mode,
    pub performance: BTreeMap<String, Binding>,
    pub desktop: BTreeMap<String, Binding>,
    pub scroll_speed: f32,
}
pub const EFFECT_NAMES: [&str; 8] = [
    "reverb",
    "delay",
    "cutoff",
    "resonance",
    "chorus",
    "drive",
    "width",
    "tremolo",
];
impl Default for ControllerSettings {
    fn default() -> Self {
        let mut performance = BTreeMap::new();
        for (i, effect) in EFFECT_NAMES.iter().enumerate() {
            performance.insert(format!("encoder-{}", i + 1), Binding::new("effect", effect));
        }
        for (control, action, value) in [
            ("btn.encoderUp", "sound", "-1"),
            ("btn.encoderDown", "sound", "1"),
            ("btn.padUp", "kit", "-1"),
            ("btn.padDown", "kit", "1"),
            ("btn.trackPrevious", "lighting", "-1"),
            ("btn.trackNext", "lighting", "1"),
            ("btn.undo", "undo", ""),
            ("btn.play", "loopPlay", ""),
            ("btn.stop", "loopStop", ""),
            ("btn.record", "loopRecord", ""),
            ("btn.loop", "loopOverdub", ""),
            ("fbtn.9", "mode", ""),
        ] {
            performance.insert(control.into(), Binding::new(action, value));
        }
        for i in 1..=8 {
            performance.insert(
                format!("fbtn.{i}"),
                Binding::new("favorite", &(i - 1).to_string()),
            );
        }
        let desktop = [
            ("fbtn.9", "mode", ""),
            ("btn.undo", "shortcut", "Ctrl+Z"),
            ("btn.play", "shortcut", "MediaPlayPause"),
            ("btn.stop", "shortcut", "MediaStop"),
            ("btn.trackPrevious", "shortcut", "MediaPrevious"),
            ("btn.trackNext", "shortcut", "MediaNext"),
            ("pitch-wheel", "scroll", ""),
            ("fbtn.1", "shortcut", "VolumeDown"),
            ("fbtn.2", "shortcut", "VolumeUp"),
            ("fbtn.3", "shortcut", "VolumeMute"),
        ]
        .into_iter()
        .map(|(c, a, v)| (c.into(), Binding::new(a, v)))
        .collect();
        Self {
            enabled: true,
            mode: Mode::Performance,
            performance,
            desktop,
            scroll_speed: 1.,
        }
    }
}
impl ControllerSettings {
    pub fn bindings(&self) -> &BTreeMap<String, Binding> {
        if self.mode == Mode::Performance {
            &self.performance
        } else {
            &self.desktop
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(0.1..=4.).contains(&self.scroll_speed)
            || self.performance.len() > 256
            || self.desktop.len() > 256
        {
            return Err("コントローラー設定が範囲外です".into());
        }
        for (id, b) in self.performance.iter().chain(&self.desktop) {
            if id.is_empty()
                || id.len() > 100
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".:-_".contains(&b))
                || b.value.len() > 4096
            {
                return Err("割り当てが不正です".into());
            }
            let valid = match b.action.as_str() {
                "none" | "mode" | "undo" | "loopPlay" | "loopStop" | "loopRecord"
                | "loopOverdub" | "loopClear" | "piano" | "panic" | "volume" | "brightness"
                | "scroll" => true,
                "effect" => EFFECT_NAMES.contains(&b.value.as_str()),
                "sound" | "kit" | "lighting" => ["-1", "1"].contains(&b.value.as_str()),
                "favorite" => b.value.parse::<usize>().is_ok_and(|v| v < 128),
                "shortcut" => shortcut(&b.value).is_ok(),
                "open" => valid_target(&b.value),
                _ => false,
            };
            if !valid {
                return Err(format!("{id} の割り当てを確認してください"));
            }
        }
        Ok(())
    }
}
pub fn valid_target(target: &str) -> bool {
    if target.chars().any(char::is_control) {
        return false;
    }
    if target.starts_with("https://") || target.starts_with("http://") {
        return target
            .split_once("://")
            .is_some_and(|(_, host)| !host.is_empty() && !host.starts_with('/'));
    }
    let p = std::path::Path::new(target);
    p.is_absolute() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}
pub fn shortcut(text: &str) -> Result<Vec<u16>, String> {
    let keys: Vec<_> = text.split('+').map(str::trim).collect();
    if keys.is_empty() || keys.len() > 5 {
        return Err("ショートカットは最大5キーです".into());
    }
    let mut result = Vec::new();
    for key in keys {
        let upper = key.to_ascii_uppercase();
        let code = match upper.as_str() {
            "CTRL" => 0x11,
            "SHIFT" => 0x10,
            "ALT" => 0x12,
            "WIN" => 0x5b,
            "ENTER" => 0x0d,
            "ESCAPE" | "ESC" => 0x1b,
            "SPACE" => 0x20,
            "TAB" => 0x09,
            "BACKSPACE" => 8,
            "DELETE" => 0x2e,
            "LEFT" => 0x25,
            "UP" => 0x26,
            "RIGHT" => 0x27,
            "DOWN" => 0x28,
            "HOME" => 0x24,
            "END" => 0x23,
            "PAGEUP" => 0x21,
            "PAGEDOWN" => 0x22,
            "INSERT" => 0x2d,
            "VOLUMEMUTE" => 0xad,
            "VOLUMEDOWN" => 0xae,
            "VOLUMEUP" => 0xaf,
            "MEDIANEXT" => 0xb0,
            "MEDIAPREVIOUS" => 0xb1,
            "MEDIASTOP" => 0xb2,
            "MEDIAPLAYPAUSE" => 0xb3,
            _ if upper.len() == 1 && upper.as_bytes()[0].is_ascii_alphanumeric() => {
                upper.as_bytes()[0] as u16
            }
            _ if upper.starts_with('F') => upper[1..]
                .parse::<u16>()
                .ok()
                .filter(|n| (1..=24).contains(n))
                .map(|n| 0x6f + n)
                .ok_or("キー名が不正です")?,
            _ => return Err(format!("未対応のキー名: {key}")),
        };
        if result.contains(&code) {
            return Err("同じキーが重複しています".into());
        }
        result.push(code);
    }
    if result.iter().all(|c| [0x10, 0x11, 0x12, 0x5b].contains(c)) {
        return Err("修飾キーと一緒に押すキーを指定してください".into());
    }
    Ok(result)
}
#[derive(Clone, Debug)]
pub struct ControlInput {
    pub id: String,
    pub raw: String,
    pub value: f32,
    pub delta: Option<f32>,
    pub down: bool,
    pub continuous: bool,
}
pub fn decode(
    source: &str,
    b: &[u8],
    layout: &DeviceLayout,
    encoder_mode: Option<u8>,
    fader_mode: Option<u8>,
) -> Option<ControlInput> {
    if b.len() != 3 || b[1] > 127 || b[2] > 127 {
        return None;
    }
    let raw = format!("midi:{}:{}:{}", source, b[0], b[1]);
    let mut input = ControlInput {
        id: raw.clone(),
        raw,
        value: b[2] as f32 / 127.,
        delta: None,
        down: b[2] > 0,
        continuous: false,
    };
    if source == "keyboard" {
        if b[0] & 0xf0 == 0xe0 {
            input.id = "pitch-wheel".into();
            input.value = (b[1] as u16 + ((b[2] as u16) << 7)) as f32 / 16383.;
            input.continuous = true;
        } else if b[0] & 0xf0 == 0xb0 && b[1] == 1 {
            input.id = "mod-wheel".into();
            input.continuous = true;
        } else {
            return None;
        }
        return Some(input);
    }
    if source != "daw" {
        return None;
    }
    if b[0] == 0xbf {
        if (21..=28).contains(&b[1]) && encoder_mode.is_none_or(|m| [1, 2, 4].contains(&m)) {
            input.id = format!("encoder-{}", b[1] - 20);
            input.continuous = true;
        } else if (85..=92).contains(&b[1]) && encoder_mode == Some(5) {
            input.id = format!("encoder-{}", b[1] - 84);
            input.delta = Some((b[2] as f32 - 64.) / 127.);
            input.continuous = true;
        } else if (5..=13).contains(&b[1]) && fader_mode.is_none_or(|m| m == 1) {
            input.id = format!("fader-{}", b[1] - 4);
            input.continuous = true;
        } else if let Some(led) = layout.leds.iter().find(|l| {
            l.address.cc == Some(b[1])
                && (!l.id.starts_with("fbtn.") || fader_mode.is_none_or(|m| m == 1))
                && (!l.id.starts_with("btn.encoder") || encoder_mode.is_none_or(|m| m < 6))
        }) {
            input.id = led.id.clone();
        }
    } else if [0x80, 0x90].contains(&(b[0] & 0xf0)) && b[0] & 15 != 15 {
        input.raw = format!("midi:daw:{}:{}", 0x90 | (b[0] & 15), b[1]);
        input.id = input.raw.clone();
        if let Some(led) = layout.leds.iter().find(|l| {
            l.group == "pads"
                && [0, 9].contains(&(b[0] & 15))
                && if b[0] & 15 == 9 {
                    l.address.drum_note == Some(b[1])
                } else {
                    l.address.daw_note == Some(b[1])
                }
        }) {
            input.id = led.id.clone();
        }
        if b[0] & 0xf0 == 0x80 {
            input.down = false;
            input.value = 0.;
        }
    } else if b[0] & 0xf0 != 0xb0 || [0xbe, 0xb6, 0xb7].contains(&b[0]) {
        return None;
    }
    Some(input)
}
#[derive(Default)]
pub struct Edges {
    held: HashSet<String>,
}
impl Edges {
    pub fn press(&mut self, input: &ControlInput) -> bool {
        if input.continuous {
            return true;
        }
        if input.down {
            self.held.insert(input.id.clone())
        } else {
            self.held.remove(&input.id);
            false
        }
    }
    pub fn clear(&mut self) {
        self.held.clear();
    }
}
pub fn scroll_rate(value: f32, speed: f32) -> f32 {
    let centered = (value * 2. - 1.).clamp(-1., 1.);
    if centered.abs() < 0.12 {
        0.
    } else {
        centered.signum() * ((centered.abs() - 0.12) / 0.88).powi(2) * 2400. * speed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_validate_and_shortcuts_reject_ambiguous_input() {
        assert!(ControllerSettings::default().validate().is_ok());
        assert_eq!(shortcut("Ctrl+Shift+Z").unwrap(), vec![17, 16, 90]);
        assert!(shortcut("Ctrl").is_err());
        assert!(shortcut("Ctrl+Ctrl+Z").is_err());
        assert!(shortcut("F25").is_err());
        assert!(!valid_target("javascript:alert(1)"));
    }
    #[test]
    fn modes_touches_and_button_edges_are_distinct() {
        let l = DeviceLayout::default();
        let mut edges = Edges::default();
        let down = decode("daw", &[0xbf, 77, 127], &l, Some(2), Some(1)).unwrap();
        assert_eq!(down.id, "btn.undo");
        assert!(edges.press(&down));
        assert!(!edges.press(&down));
        let up = decode("daw", &[0xbf, 77, 0], &l, None, None).unwrap();
        assert!(!edges.press(&up));
        assert!(edges.press(&down));
        assert!(decode("daw", &[0xbe, 21, 127], &l, None, None).is_none());
        assert!(decode("daw", &[0xb6, 30, 2], &l, None, None).is_none());
        assert!(decode("daw", &[0xbf, 21, 100], &l, Some(6), None)
            .unwrap()
            .id
            .starts_with("midi:"));
        assert_eq!(
            decode("daw", &[0xbf, 85, 63], &l, Some(5), None)
                .unwrap()
                .delta,
            Some(-1. / 127.)
        );
        assert_eq!(
            decode("daw", &[0xbf, 21, 110], &l, Some(4), None)
                .unwrap()
                .id,
            "encoder-1"
        );
        assert!(decode("daw", &[0xbf, 51, 127], &l, Some(6), None)
            .unwrap()
            .id
            .starts_with("midi:"));
        assert_eq!(
            decode("daw", &[0xb0, 20, 110], &l, Some(6), None)
                .unwrap()
                .id,
            "midi:daw:176:20"
        );
    }
    #[test]
    fn pitch_has_a_dead_zone_and_bounded_continuous_rate() {
        assert_eq!(scroll_rate(8192. / 16383., 1.), 0.);
        assert_eq!(scroll_rate(0.54, 4.), 0.);
        assert_eq!(scroll_rate(1., 1.), 2400.);
        assert_eq!(scroll_rate(0., 1.), -2400.);
    }
}
