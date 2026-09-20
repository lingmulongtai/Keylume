use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PianoSettings {
    pub enabled: bool,
    pub sound: String,
    pub drums: bool,
    pub drum_kit: crate::drums::DrumSettings,
    pub drum_volume: f32,
    pub volume: f32,
    pub volume_fader: u8,
    pub octave: i8,
    pub output_device: String,
    pub buffer_frames: u32,
    pub mute_with_daw: bool,
    pub effects: crate::instrument_fx::InstrumentFx,
    pub favorites: Vec<String>,
}
impl Default for PianoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sound: "upright".into(),
            drums: true,
            drum_kit: Default::default(),
            drum_volume: 0.7,
            volume: 0.5,
            volume_fader: 9,
            octave: 0,
            output_device: String::new(),
            buffer_frames: 256,
            mute_with_daw: true,
            effects: Default::default(),
            favorites: [
                "upright",
                "bright",
                "fm-piano",
                "honky-tonk",
                "generaluser:0:0",
                "generaluser:0:81",
                "generaluser:0:89",
                "generaluser:0:48",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    }
}
impl PianoSettings {
    pub fn validate(&self) -> Result<(), String> {
        self.effects.validate()?;
        self.drum_kit.validate()?;
        if self.favorites.len() > 128
            || self
                .favorites
                .iter()
                .any(|id| crate::sound_library::SoundId::parse(id).is_err())
        {
            return Err("音源のお気に入りが不正です".into());
        }
        crate::sound_library::SoundId::parse(&self.sound)?;
        if !(0.0..=1.0).contains(&self.volume)
            || !(0.0..=1.0).contains(&self.drum_volume)
            || self.volume_fader > 9
            || !(-3..=3).contains(&self.octave)
            || ![128, 256, 512, 1024].contains(&self.buffer_frames)
            || self.output_device.len() > 512
        {
            Err("ピアノの設定値が範囲外です".into())
        } else {
            Ok(())
        }
    }
}
/// The factory DAW Volume faders send CC5..13 on channel 16. Zero disables the binding.
pub fn fader_volume(selected: u8, source: &str, bytes: &[u8]) -> Option<f32> {
    (source == "daw"
        && (1..=9).contains(&selected)
        && bytes.len() == 3
        && bytes[0] == 0xbf
        && bytes[1] == selected + 4
        && bytes[2] <= 127)
        .then(|| bytes[2] as f32 / 127.)
}
pub fn sound_font() -> Result<Arc<SoundFont>, String> {
    sound_font_for("upright")
}
pub fn sound_font_for(id: &str) -> Result<Arc<SoundFont>, String> {
    let bytes: &[u8] = match id {
        "upright" => include_bytes!("../../resources/piano/upright.sf2"),
        "bright" => include_bytes!("../../resources/piano/bright.sf2"),
        "fm-piano" => include_bytes!("../../resources/piano/fm-piano.sf2"),
        "honky-tonk" => include_bytes!("../../resources/piano/honky-tonk.sf2"),
        _ => return Err("音源が見つかりません".into()),
    };
    SoundFont::new(&mut Cursor::new(bytes))
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub struct PianoSynth {
    synth: Synthesizer,
    // Track screen and physical notes separately, including their original MIDI channel.
    held: [[[Option<u8>; 128]; 16]; 2],
    octave: i8,
    patch: (u16, u8),
}
impl PianoSynth {
    pub fn new(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Self, String> {
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.maximum_polyphony = 128;
        settings.enable_reverb_and_chorus = false;
        let mut synth = Synthesizer::new(font, &settings).map_err(|e| e.to_string())?;
        synth.set_master_volume(0.7);
        let mut instrument = Self {
            synth,
            held: [[[None; 128]; 16]; 2],
            octave: 0,
            patch: (0, 0),
        };
        instrument.apply_patch();
        Ok(instrument)
    }
    pub fn panic(&mut self) {
        self.synth.reset();
        self.apply_patch();
        self.held = [[[None; 128]; 16]; 2];
    }
    fn apply_patch(&mut self) {
        for channel in 0..16 {
            // RustySynth adds128 to channel10's bank; explicitly select the same instrument on all inputs.
            self.synth.process_midi_message(
                channel,
                0xb0,
                0,
                self.patch.0 as i32 - if channel == 9 { 128 } else { 0 },
            );
            self.synth
                .process_midi_message(channel, 0xc0, self.patch.1 as i32, 0);
        }
    }
    pub fn set_patch(&mut self, bank: u16, program: u8) {
        if self.patch != (bank, program) {
            self.patch = (bank, program);
            self.panic();
        }
    }
    pub fn set_octave(&mut self, octave: i8) {
        if self.octave != octave {
            self.panic();
            self.octave = octave;
        }
    }
    pub fn midi(&mut self, screen: bool, b: [u8; 3]) {
        if screen && b == [0, 0, 0] {
            for ch in 0..16 {
                for key in 0..128 {
                    if let Some(note) = self.held[1][ch][key].take() {
                        if self.held[0][ch][key].is_none() {
                            self.synth.note_off(ch as i32, note as i32);
                        }
                    }
                }
            }
            return;
        }
        if b[1] > 127 || b[2] > 127 {
            return;
        }
        let source = usize::from(screen);
        let ch = (b[0] & 15) as usize;
        let key = b[1] as usize;
        match b[0] & 0xf0 {
            0x90 if b[2] > 0 => {
                let transposed = b[1] as i16 + self.octave as i16 * 12;
                if !(0..=127).contains(&transposed) {
                    return;
                }
                self.held[source][ch][key] = Some(transposed as u8);
                self.synth
                    .note_on(ch as i32, transposed as i32, b[2] as i32);
            }
            0x80 | 0x90 => {
                if let Some(note) = self.held[source][ch][key].take() {
                    if self.held[1 - source][ch][key].is_none() {
                        self.synth.note_off(ch as i32, note as i32);
                    }
                }
            }
            // Keep the bundled piano patch; program/bank changes do not replace it.
            0xb0 if [1, 7, 10, 11, 64, 120, 121, 123].contains(&b[1]) => {
                self.synth
                    .process_midi_message(ch as i32, 0xb0, b[1] as i32, b[2] as i32);
                if b[1] == 120 || b[1] == 123 {
                    for source in &mut self.held {
                        source[ch] = [None; 128];
                    }
                }
            }
            0xe0 => self
                .synth
                .process_midi_message(ch as i32, 0xe0, b[1] as i32, b[2] as i32),
            _ => {}
        }
    }
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_bundled_sounds_render_and_old_settings_keep_the_upright() {
        for id in ["upright", "bright", "fm-piano", "honky-tonk"] {
            let font = sound_font_for(id).unwrap();
            assert!(font
                .get_presets()
                .iter()
                .any(|p| p.get_patch_number() == 0 && p.get_bank_number() == 0));
            let mut synth = PianoSynth::new(&font, 48000).unwrap();
            for note in [48, 60, 72] {
                synth.midi(false, [0x90, note, 100]);
            }
            assert!(energy(&mut synth, 12000) > 0.000001, "{id}");
        }
        assert!(sound_font_for("unknown").is_err());
        let old: PianoSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.sound, "upright");
    }
    #[test]
    fn only_the_selected_daw_fader_changes_volume() {
        assert_eq!(fader_volume(9, "daw", &[0xbf, 13, 127]), Some(1.));
        assert_eq!(fader_volume(3, "daw", &[0xbf, 7, 0]), Some(0.));
        for (selected, source, b) in [
            (0, "daw", vec![0xbf, 4, 127]),
            (3, "keyboard", vec![0xbf, 7, 100]),
            (3, "daw", vec![0xb0, 7, 100]),
            (3, "daw", vec![0xbf, 13, 100]),
            (3, "daw", vec![0xbf, 7]),
            (3, "daw", vec![0xbf, 7, 255]),
        ] {
            assert_eq!(fader_volume(selected, source, &b), None);
        }
        let old: PianoSettings = serde_json::from_str(r#"{"volume":0.3}"#).unwrap();
        assert_eq!(old.volume_fader, 9);
    }
    fn energy(s: &mut PianoSynth, frames: usize) -> f32 {
        let mut left = vec![0.; frames];
        let mut right = left.clone();
        s.render(&mut left, &mut right);
        assert!(left.iter().chain(&right).all(|x| x.is_finite()));
        left.iter().map(|x| x * x).sum::<f32>() / frames as f32
    }
    #[test]
    fn bundled_piano_is_audible_velocity_sensitive_and_pedal_releases() {
        let font = sound_font().unwrap();
        let mut s = PianoSynth::new(&font, 48000).unwrap();
        assert_eq!(energy(&mut s, 480), 0.);
        s.midi(false, [0x90, 60, 35]);
        let soft = energy(&mut s, 12000);
        s.panic();
        s.midi(false, [0x90, 60, 110]);
        let loud = energy(&mut s, 12000);
        assert!(
            loud > soft * 2. && soft > 0.0000001,
            "soft {soft}, loud {loud}"
        );
        s.midi(false, [0xb0, 64, 127]);
        s.midi(false, [0x80, 60, 0]);
        let sustained = energy(&mut s, 24000);
        assert!(sustained > 0.0000001);
        s.midi(false, [0xb0, 64, 0]);
        energy(&mut s, 96000);
        assert!(energy(&mut s, 12000) < sustained * 0.01);
        s.panic();
        assert_eq!(energy(&mut s, 480), 0.);
    }
    #[test]
    fn octave_changes_and_out_of_range_notes_cannot_leave_stuck_voices() {
        let mut s = PianoSynth::new(&sound_font().unwrap(), 44100).unwrap();
        s.set_octave(1);
        s.midi(false, [0x90, 60, 100]);
        assert_eq!(s.held[0][0][60], Some(72));
        s.set_octave(-1);
        assert_eq!(energy(&mut s, 1024), 0.);
        s.midi(false, [0x80, 60, 0]);
        s.midi(false, [0x90, 0, 100]);
        assert_eq!(energy(&mut s, 1024), 0.);
        s.midi(false, [0x91, 60, 100]);
        s.midi(true, [0x91, 60, 100]);
        s.midi(false, [0x91, 60, 0]);
        assert!(energy(&mut s, 4096) > 0.);
        s.midi(true, [0, 0, 0]);
        energy(&mut s, 88200);
        assert!(energy(&mut s, 4096) < 0.0000001);
    }
}
