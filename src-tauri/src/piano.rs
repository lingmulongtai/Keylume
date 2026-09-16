use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PianoSettings {
    pub enabled: bool,
    pub volume: f32,
    pub octave: i8,
    pub output_device: String,
    pub buffer_frames: u32,
    pub mute_with_daw: bool,
}
impl Default for PianoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.5,
            octave: 0,
            output_device: String::new(),
            buffer_frames: 256,
            mute_with_daw: true,
        }
    }
}
impl PianoSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.volume)
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
pub fn sound_font() -> Result<Arc<SoundFont>, String> {
    SoundFont::new(&mut Cursor::new(
        include_bytes!("../../resources/piano/upright.sf2").as_slice(),
    ))
    .map(Arc::new)
    .map_err(|e| e.to_string())
}

pub struct PianoSynth {
    synth: Synthesizer,
    // Track screen and physical notes separately, including their original MIDI channel.
    held: [[[Option<u8>; 128]; 16]; 2],
    octave: i8,
}
impl PianoSynth {
    pub fn new(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Self, String> {
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.maximum_polyphony = 128;
        settings.enable_reverb_and_chorus = false;
        let mut synth = Synthesizer::new(font, &settings).map_err(|e| e.to_string())?;
        synth.set_master_volume(0.7);
        Ok(Self {
            synth,
            held: [[[None; 128]; 16]; 2],
            octave: 0,
        })
    }
    pub fn panic(&mut self) {
        self.synth.reset();
        self.held = [[[None; 128]; 16]; 2];
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
