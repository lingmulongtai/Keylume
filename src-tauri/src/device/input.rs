use crate::model::DeviceLayout;
use serde::Serialize;
use std::collections::BTreeMap;

/// Observed MIDI state only: no invented initial fader or encoder positions.
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputState {
    pub encoders: [Option<i32>; 8],
    pub relative: [bool; 8],
    pub faders: [Option<u8>; 9],
    pub pitch: Option<u16>,
    pub modulation: Option<u8>,
    pub sustain: Option<u8>,
    pub pressure: Option<u8>,
    pub poly_pressure: BTreeMap<String, u8>,
    pub features: BTreeMap<u8, u8>,
    pub held: Vec<String>,
    pub encoder_mode: Option<u8>,
    pub fader_mode: Option<u8>,
    pub last_note: Option<[u8; 2]>,
    pub last_message: String,
    pub pulse: Option<(String, u64)>,
    #[serde(skip)]
    presses: BTreeMap<(String, u8, u8), String>,
    #[serde(skip)]
    pressure_source: String,
}
impl InputState {
    pub fn clear_port(&mut self, source: &str) {
        self.presses.retain(|(s, _, _), _| s != source);
        self.held = self.presses.values().cloned().collect();
        self.held.sort();
        self.held.dedup();
        if source == "daw" {
            self.encoders = [None; 8];
            self.relative = [false; 8];
            self.faders = [None; 9];
            self.encoder_mode = None;
            self.fader_mode = None;
            self.features.clear();
            self.poly_pressure.retain(|id, _| id.starts_with("key."));
        } else if source == "keyboard" {
            self.pitch = None;
            self.modulation = None;
            self.sustain = None;
            self.poly_pressure.retain(|id, _| !id.starts_with("key."));
        }
        if self.pressure_source == source {
            self.pressure = None;
            self.pressure_source.clear();
        }
    }
    pub fn receive(&mut self, source: &str, b: &[u8], layout: &DeviceLayout) {
        if source == "keyboard" && matches!(b, [0xfa] | [0xfb] | [0xfc]) {
            let serial = self.pulse.as_ref().map_or(1, |p| p.1.wrapping_add(1));
            self.pulse = Some((
                if b[0] == 0xfc { "btn.stop" } else { "btn.play" }.into(),
                serial,
            ));
            self.last_message = format!("{source} · {:02X}", b[0]);
            return;
        }
        if b.len() < 2 || b[0] < 0x80 || b[0] >= 0xf0 || b[1..].iter().any(|v| *v > 127) {
            return;
        }
        let kind = b[0] & 0xf0;
        if b.len() != if kind == 0xd0 || kind == 0xc0 { 2 } else { 3 } {
            return;
        }
        let ch = b[0] & 15;
        let v = *b.get(2).unwrap_or(&0);
        let keyboard = source == "keyboard" || source == "screen";
        self.last_message = format!(
            "{source} · {}",
            b.iter()
                .map(|v| format!("{v:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        if kind == 0x90 || kind == 0x80 || kind == 0xa0 {
            let id = if keyboard {
                Some(format!("key.{}", b[1]))
            } else if source == "daw" {
                layout
                    .leds
                    .iter()
                    .find(|l| {
                        l.group == "pads"
                            && if ch == 9 {
                                l.address.drum_note == Some(b[1])
                            } else {
                                l.address.daw_note == Some(b[1])
                            }
                    })
                    .map(|l| l.id.clone())
            } else {
                None
            };
            if let Some(id) = id {
                if kind == 0xa0 {
                    self.poly_pressure.insert(id, v);
                } else {
                    let key = (source.into(), ch, b[1]);
                    if kind == 0x90 && v > 0 {
                        self.presses.insert(key, id);
                        if keyboard {
                            self.last_note = Some([b[1], v]);
                        }
                    } else {
                        self.presses.remove(&key);
                        self.poly_pressure.remove(&id);
                    }
                }
            }
        } else if source == "daw" && kind == 0xd0 {
            self.pressure = Some(b[1]);
            self.pressure_source = source.into();
        } else if source == "daw" && b[0] == 0xbe {
            let id = match b[1] {
                5..=13 => Some(format!("fader-{}", b[1] - 4)),
                21..=28 => Some(format!("encoder-{}", b[1] - 20)),
                85..=92 => Some(format!("encoder-{}", b[1] - 84)),
                _ => None,
            };
            if let Some(id) = id {
                let key = (source.into(), ch, b[1]);
                if v > 0 {
                    self.presses.insert(key, id);
                } else {
                    self.presses.remove(&key);
                }
            }
        } else if keyboard && b[0] != 0xbf {
            match kind {
                0xe0 => self.pitch = Some(b[1] as u16 + ((v as u16) << 7)),
                0xd0 => {
                    self.pressure = Some(b[1]);
                    self.pressure_source = source.into();
                }
                0xb0 => match b[1] {
                    1 => self.modulation = Some(v),
                    64 => self.sustain = Some(v),
                    120 | 123 => self.presses.retain(|(s, c, _), _| s != source || *c != ch),
                    121 => {
                        self.sustain = Some(0);
                        self.pitch = Some(8192);
                        self.modulation = Some(0);
                    }
                    _ => {}
                },
                _ => {}
            }
        } else if (source == "daw" || source == "keyboard") && b[0] == 0xbf {
            match b[1] {
                5..=13 => self.faders[(b[1] - 5) as usize] = Some(v),
                21..=28 => {
                    let i = (b[1] - 21) as usize;
                    self.encoders[i] = Some(v as i32);
                    self.relative[i] = false;
                }
                85..=92 => {
                    let i = (b[1] - 85) as usize;
                    let old = if self.relative[i] {
                        self.encoders[i].unwrap_or(0)
                    } else {
                        0
                    };
                    self.encoders[i] = Some((old + v as i32 - 64).rem_euclid(128));
                    self.relative[i] = true;
                }
                _ => {
                    let id = if b[1] == 63 {
                        Some("btn.shift".to_string())
                    } else {
                        layout
                            .leds
                            .iter()
                            .find(|l| l.address.cc == Some(b[1]))
                            .map(|l| l.id.clone())
                    };
                    if let Some(id) = id {
                        let key = (source.into(), ch, b[1]);
                        if v > 0 {
                            let serial = self.pulse.as_ref().map_or(1, |p| p.1.wrapping_add(1));
                            self.pulse = Some((id.clone(), serial));
                            self.presses.insert(key, id);
                        } else {
                            self.presses.remove(&key);
                        }
                    }
                }
            }
        } else if source == "daw" && b[0] == 0xb6 {
            self.features.insert(b[1], v);
            match b[1] {
                0x3f => {
                    let key = (source.into(), ch, b[1]);
                    if v > 0 {
                        self.presses.insert(key, "btn.shift".into());
                    } else {
                        self.presses.remove(&key);
                    }
                }
                0x1e => {
                    if self.encoder_mode != Some(v) {
                        self.encoders = [None; 8];
                        self.relative = [false; 8];
                    }
                    self.encoder_mode = Some(v);
                }
                0x1f => {
                    if self.fader_mode != Some(v) {
                        self.faders = [None; 9];
                    }
                    self.fader_mode = Some(v);
                }
                _ => {}
            }
        }
        self.held = self.presses.values().cloned().collect();
        self.held.sort();
        self.held.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_buttons_and_transport_are_visible_without_sticking() {
        let mut s = InputState::default();
        let l = DeviceLayout::default();
        for (i, cc) in [103, 102, 77, 117, 76, 74, 75].into_iter().enumerate() {
            s.receive("keyboard", &[0xbf, cc, 127], &l);
            assert_eq!(s.held.len(), 1);
            let id = s.held[0].clone();
            s.receive("keyboard", &[0xbf, cc, 0], &l);
            assert!(s.held.is_empty());
            assert_eq!(s.pulse, Some((id, i as u64 + 1)));
        }
        s.receive("keyboard", &[0xfa], &l);
        assert_eq!(s.pulse, Some(("btn.play".into(), 8)));
        s.receive("keyboard", &[0xfa], &l);
        assert_eq!(s.pulse, Some(("btn.play".into(), 9)));
        s.receive("keyboard", &[0xfc], &l);
        assert_eq!(s.pulse, Some(("btn.stop".into(), 10)));
        assert!(s.held.is_empty());
    }
    #[test]
    fn releasing_daw_preserves_ongoing_keyboard_and_screen_input() {
        let mut s = InputState::default();
        let l = DeviceLayout::default();
        for (source, b) in [
            ("keyboard", vec![0x90, 60, 100]),
            ("keyboard", vec![0xb0, 64, 127]),
            ("keyboard", vec![0xd0, 80]),
            ("screen", vec![0x90, 64, 100]),
            ("daw", vec![0xbf, 115, 127]),
            ("daw", vec![0xbe, 5, 127]),
            ("daw", vec![0xbf, 5, 100]),
        ] {
            s.receive(source, &b, &l);
        }
        s.clear_port("daw");
        assert_eq!(s.held, ["key.60", "key.64"]);
        assert_eq!(s.sustain, Some(127));
        assert_eq!(s.pressure, Some(80));
        assert_eq!(s.faders[0], None);
        s.clear_port("keyboard");
        assert_eq!(s.held, ["key.64"]);
        assert_eq!(s.sustain, None);
        assert_eq!(s.pressure, None);
    }
    #[test]
    fn controls_are_port_and_channel_aware() {
        let mut s = InputState::default();
        let l = DeviceLayout::default();
        s.receive("keyboard", &[0xb0, 5, 127], &l);
        s.receive("daw", &[0xb6, 5, 127], &l);
        assert_eq!(s.faders[0], None);
        s.receive("daw", &[0xbf, 5, 127], &l);
        s.receive("daw", &[0xbf, 21, 32], &l);
        assert_eq!(s.faders[0], Some(127));
        assert_eq!(s.encoders[0], Some(32));
        s.receive("daw", &[0xbf, 85, 65], &l);
        s.receive("daw", &[0xbf, 85, 63], &l);
        assert_eq!(s.encoders[0], Some(0));
        assert!(s.relative[0]);
        s.receive("keyboard", &[0xe2, 0, 64], &l);
        s.receive("keyboard", &[0xb2, 64, 127], &l);
        assert_eq!(s.pitch, Some(8192));
        assert_eq!(s.sustain, Some(127));
    }
    #[test]
    fn note_off_does_not_release_another_channel_or_source() {
        let mut s = InputState::default();
        let l = DeviceLayout::default();
        for (source, b) in [
            ("keyboard", [0x90, 60, 90]),
            ("screen", [0x90, 60, 100]),
            ("keyboard", [0x91, 60, 90]),
            ("keyboard", [0x90, 60, 0]),
            ("screen", [0x80, 60, 0]),
        ] {
            s.receive(source, &b, &l);
        }
        assert_eq!(s.held, ["key.60"]);
        s.receive("keyboard", &[0xb1, 123, 0], &l);
        assert!(s.held.is_empty());
        s.receive("daw", &[0xbf, 115, 127], &l);
        assert!(s.held.contains(&"btn.play".into()));
        s.receive("daw", &[0xbf, 115, 0], &l);
        assert!(s.held.is_empty());
    }
    #[test]
    fn touches_and_feature_replies_are_separate_from_positions_and_buttons() {
        let mut s = InputState::default();
        let l = DeviceLayout::default();
        s.receive("daw", &[0xbe, 5, 127], &l);
        assert!(s.held.contains(&"fader-1".into()));
        assert_eq!(s.faders[0], None);
        s.receive("daw", &[0xbe, 5, 0], &l);
        assert!(!s.held.contains(&"fader-1".into()));
        s.receive("daw", &[0xb6, 74, 1], &l);
        assert_eq!(s.features.get(&74), Some(&1));
        assert!(!s.held.contains(&"btn.capture".into()));
        s.receive("daw", &[0xb6, 63, 127], &l);
        assert!(s.held.contains(&"btn.shift".into()));
        s.receive("daw", &[0xdf, 85], &l);
        assert_eq!(s.pressure, Some(85));
    }
}
