use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundEvent {
    Piano(bool, [u8; 3]),
    Drum(u8, u8),
    Release,
}
#[derive(Clone, Copy)]
struct Recorded {
    beat: f64,
    event: SoundEvent,
    order: usize,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopConfig {
    pub bpm: f64,
    pub bars: u8,
    pub metronome: bool,
}
impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            bpm: 100.,
            bars: 2,
            metronome: true,
        }
    }
}
impl LoopConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(40.0..=240.).contains(&self.bpm) || ![1, 2, 4, 8].contains(&self.bars) {
            Err("テンポは40–240、小節数は1・2・4・8です".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy)]
pub enum LoopCommand {
    Record(LoopConfig),
    Play,
    Overdub,
    Stop,
    Clear,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopStatus {
    pub mode: String,
    pub beat: f64,
    pub beats: f64,
    pub count: usize,
    pub bpm: f64,
    pub metronome: bool,
    pub full: bool,
}
pub struct Looper {
    events: Vec<Recorded>,
    pending: Vec<Recorded>,
    pub config: LoopConfig,
    pub mode: u8,
    pub beat: f64,
    cursor: usize,
    last_click: i32,
    held: [[[Option<u8>; 128]; 16]; 2],
    pub full: bool,
}
impl Default for Looper {
    fn default() -> Self {
        Self {
            events: Vec::with_capacity(8192),
            pending: Vec::with_capacity(8192),
            config: LoopConfig::default(),
            mode: 0,
            beat: 0.,
            cursor: 0,
            last_click: i32::MIN,
            held: [[[None; 128]; 16]; 2],
            full: false,
        }
    }
}
impl Looper {
    pub fn command(&mut self, c: LoopCommand) {
        match c {
            LoopCommand::Record(config) => {
                self.config = config;
                self.events.clear();
                self.pending.clear();
                self.mode = 1;
                self.beat = -4.;
                self.full = false;
            }
            LoopCommand::Play => {
                if !self.events.is_empty() {
                    self.mode = 3;
                    self.beat = 0.;
                }
            }
            LoopCommand::Overdub => {
                if self.mode == 3 {
                    self.mode = 4;
                } else if self.mode == 4 {
                    self.merge();
                    self.mode = 3;
                }
            }
            LoopCommand::Stop => {
                self.merge();
                self.mode = 0;
                self.beat = 0.;
            }
            LoopCommand::Clear => {
                self.events.clear();
                self.pending.clear();
                self.mode = 0;
                self.beat = 0.;
                self.full = false;
            }
        }
        self.cursor = 0;
        self.last_click = i32::MIN;
        self.held = [[[None; 128]; 16]; 2];
    }
    fn merge(&mut self) {
        self.events.append(&mut self.pending);
        self.events
            .sort_unstable_by(|a, b| a.beat.total_cmp(&b.beat).then(a.order.cmp(&b.order)));
        self.cursor = self.events.partition_point(|e| e.beat < self.beat);
    }
    pub fn capture(&mut self, event: SoundEvent, octave: i8) {
        if ![2, 4].contains(&self.mode) || event == SoundEvent::Release {
            return;
        }
        if event == SoundEvent::Piano(true, [0, 0, 0]) {
            for channel in 0..16 {
                for key in 0..128 {
                    if self.held[1][channel][key].is_some() {
                        self.capture(
                            SoundEvent::Piano(true, [0x80 | channel as u8, key as u8, 0]),
                            octave,
                        );
                    }
                }
            }
            return;
        }
        if self.events.len() + self.pending.len() >= 8192 {
            self.full = true;
            return;
        }
        let event = match event {
            SoundEvent::Piano(screen, mut b) => {
                let source = usize::from(screen);
                let ch = (b[0] & 15) as usize;
                let key = b[1] as usize;
                if key > 127 || b[2] > 127 {
                    return;
                }
                match b[0] & 0xf0 {
                    0x90 if b[2] > 0 => {
                        let pitch = b[1] as i16 + octave as i16 * 12;
                        if !(0..=127).contains(&pitch) {
                            return;
                        }
                        self.held[source][ch][key] = Some(pitch as u8);
                        b[1] = pitch as u8;
                    }
                    0x80 | 0x90 => {
                        let Some(pitch) = self.held[source][ch][key].take() else {
                            return;
                        };
                        b[1] = pitch;
                    }
                    0xb0 | 0xe0 => {}
                    _ => return,
                }
                SoundEvent::Piano(screen, b)
            }
            e => e,
        };
        self.pending.push(Recorded {
            beat: self.beat.max(0.),
            event,
            order: self.count(),
        });
    }
    /// Returns true on a loop boundary, so playback voices can release before the next lap.
    pub fn advance(&mut self, seconds: f64, out: &mut Vec<SoundEvent>) -> bool {
        out.clear();
        if self.mode == 0 {
            return false;
        }
        let before = self.beat;
        self.beat += seconds * self.config.bpm / 60.;
        let mut reset = false;
        let length = self.config.bars as f64 * 4.;
        if self.mode == 1 && self.beat >= 0. {
            self.beat = 0.;
            self.mode = 2;
            self.cursor = 0;
            reset = true;
            out.push(SoundEvent::Release);
        }
        if self.beat >= length {
            if self.mode >= 3 {
                while self.cursor < self.events.len() {
                    out.push(self.events[self.cursor].event);
                    self.cursor += 1;
                }
            }
            out.push(SoundEvent::Release);
            self.beat -= length;
            self.merge();
            if self.mode == 2 {
                self.mode = 3;
            }
            self.cursor = 0;
            self.held = [[[None; 128]; 16]; 2];
            reset = true;
        }
        if self.mode >= 3 {
            while self.cursor < self.events.len() && self.events[self.cursor].beat <= self.beat {
                if reset || self.events[self.cursor].beat >= before {
                    out.push(self.events[self.cursor].event);
                }
                self.cursor += 1;
            }
        }
        let click = self.beat.floor() as i32;
        if self.config.metronome && click != self.last_click {
            self.last_click = click;
            out.push(SoundEvent::Drum(
                16,
                if click.rem_euclid(4) == 0 { 80 } else { 45 },
            ));
        }
        reset
    }
    pub fn count(&self) -> usize {
        self.events.len() + self.pending.len()
    }
    pub fn status(&self) -> LoopStatus {
        LoopStatus {
            mode: ["stopped", "countIn", "recording", "playing", "overdub"][self.mode as usize]
                .into(),
            beat: self.beat,
            beats: self.config.bars as f64 * 4.,
            count: self.events.len() + self.pending.len(),
            bpm: self.config.bpm,
            metronome: self.config.metronome,
            full: self.full,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_transposed_notes_and_drums_after_count_in_then_loops() {
        let mut l = Looper::default();
        let mut out = Vec::with_capacity(8193);
        l.command(LoopCommand::Record(LoopConfig {
            bpm: 120.,
            bars: 1,
            metronome: false,
        }));
        l.capture(SoundEvent::Drum(8, 100), 0);
        assert_eq!(l.status().count, 0);
        l.advance(2., &mut out);
        assert_eq!(l.mode, 2);
        l.capture(SoundEvent::Piano(false, [0x90, 60, 100]), 1);
        l.capture(SoundEvent::Drum(8, 100), 0);
        l.advance(0.5, &mut out);
        l.capture(SoundEvent::Piano(false, [0x80, 60, 0]), 0);
        assert!(l.advance(1.5, &mut out));
        assert_eq!(l.mode, 3);
        assert!(out.contains(&SoundEvent::Piano(false, [0x90, 72, 100])));
        l.advance(0.5, &mut out);
        assert!(out.contains(&SoundEvent::Piano(false, [0x80, 72, 0])));
        l.command(LoopCommand::Stop);
        assert_eq!(l.status().count, 3);
        l.command(LoopCommand::Play);
        assert_eq!(l.mode, 3);
        l.command(LoopCommand::Clear);
        assert_eq!(l.status().count, 0);
    }
    #[test]
    fn a_hit_just_before_the_boundary_is_not_lost() {
        let mut l = Looper {
            config: LoopConfig {
                bpm: 120.,
                bars: 1,
                metronome: false,
            },
            mode: 3,
            beat: 3.998,
            ..Default::default()
        };
        l.events.push(Recorded {
            beat: 3.999,
            event: SoundEvent::Drum(8, 100),
            order: 0,
        });
        let mut out = Vec::new();
        l.advance(0.002, &mut out);
        assert_eq!(out, vec![SoundEvent::Drum(8, 100), SoundEvent::Release]);
    }
    #[test]
    fn focus_loss_records_screen_note_releases() {
        let mut l = Looper::default();
        let mut out = Vec::new();
        l.command(LoopCommand::Record(LoopConfig {
            bpm: 120.,
            bars: 1,
            metronome: false,
        }));
        l.advance(2., &mut out);
        l.capture(SoundEvent::Piano(true, [0x90, 60, 100]), 0);
        l.advance(0.25, &mut out);
        l.capture(SoundEvent::Piano(true, [0, 0, 0]), 0);
        l.advance(1.75, &mut out);
        l.advance(0.25, &mut out);
        assert!(out.contains(&SoundEvent::Piano(true, [0x80, 60, 0])));
    }
    #[test]
    fn overdub_is_bounded_and_stopping_commits_the_current_layer() {
        let mut l = Looper::default();
        l.command(LoopCommand::Record(LoopConfig::default()));
        let mut out = Vec::new();
        l.advance(2.4, &mut out);
        l.capture(SoundEvent::Drum(8, 100), 0);
        l.advance(4.8, &mut out);
        l.command(LoopCommand::Overdub);
        assert_eq!(l.mode, 4);
        for _ in 0..9000 {
            l.capture(SoundEvent::Drum(9, 100), 0);
        }
        assert!(l.full);
        assert_eq!(l.status().count, 8192);
        l.command(LoopCommand::Stop);
        assert_eq!(l.status().count, 8192);
        l.command(LoopCommand::Clear);
        assert!(!l.full);
    }
}
