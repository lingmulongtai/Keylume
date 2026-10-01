use crate::performance::{PerformanceEngine, StageSettings};
use std::collections::HashMap;

#[derive(Default)]
pub struct PlaybackBatch {
    pub reset: bool,
    pub messages: Vec<[u8; 3]>,
}
#[derive(Clone, Copy)]
struct Event {
    at: f64,
    id: u32,
    pitch: u8,
    velocity: u8,
    end: f64,
}
struct Active {
    pitch: u8,
    deadline: f64,
}
pub struct SongPlayback {
    revision: u64,
    settings: Option<StageSettings>,
    epoch: Option<u64>,
    events: Vec<Event>,
    cursor: usize,
    active: HashMap<u32, Active>,
    counts: [u16; 128],
    attacks: [f64; 128],
    last_position: f64,
    stationary_since: f64,
}
impl Default for SongPlayback {
    fn default() -> Self {
        Self {
            revision: 0,
            settings: None,
            epoch: None,
            events: vec![],
            cursor: 0,
            active: HashMap::new(),
            counts: [0; 128],
            attacks: [f64::NEG_INFINITY; 128],
            last_position: f64::NEG_INFINITY,
            stationary_since: 0.,
        }
    }
}
impl SongPlayback {
    pub fn invalidate(&mut self) {
        self.epoch = None;
    }
    fn release(&mut self, id: u32, out: &mut Vec<[u8; 3]>) {
        if let Some(note) = self.active.remove(&id) {
            let count = &mut self.counts[note.pitch as usize];
            *count = count.saturating_sub(1);
            if *count == 0 {
                out.push([0x80, note.pitch, 0]);
            }
        }
    }
    fn start(&mut self, event: Event, now: f64, at: f64, speed: f64, out: &mut Vec<[u8; 3]>) {
        if event.end <= at {
            return;
        }
        self.active.insert(
            event.id,
            Active {
                pitch: event.pitch,
                deadline: now + (event.end - at) / speed,
            },
        );
        let count = &mut self.counts[event.pitch as usize];
        // Coalesce simultaneous unisons while keeping independent note lifetimes.
        if *count == 0 || (self.attacks[event.pitch as usize] - event.at).abs() > 0.001 {
            out.push([0x90, event.pitch, event.velocity]);
        }
        self.attacks[event.pitch as usize] = event.at;
        *count = count.saturating_add(1);
    }
    pub fn update(&mut self, engine: &PerformanceEngine) -> PlaybackBatch {
        let settings = &engine.settings;
        let changed = self.settings.as_ref().is_none_or(|old| {
            old.tracks != settings.tracks
                || old.practice_hand != settings.practice_hand
                || old.hand_strategy != settings.hand_strategy
                || old.split_pitch != settings.split_pitch
                || old.track_hands != settings.track_hands
                || old.practice_mode != settings.practice_mode
                || old.playback_mode != settings.playback_mode
                || old.loop_enabled != settings.loop_enabled
                || old.loop_start != settings.loop_start
                || old.loop_end != settings.loop_end
        });
        if changed || self.revision != engine.song_revision {
            self.events.clear();
            if let Some(song) = &engine.song {
                for n in &song.notes {
                    let audible = settings.playback_mode != "off"
                        && (settings.practice_mode == "listen"
                            || settings.playback_mode == "full"
                            || settings.practice_hand != "both" && !settings.practiced(n));
                    if !audible
                        || !settings.tracks.contains(&n.track)
                        || settings.loop_enabled
                            && (n.start < settings.loop_start || n.start >= settings.loop_end)
                    {
                        continue;
                    }
                    let end = if settings.loop_enabled {
                        n.end.min(settings.loop_end)
                    } else {
                        n.end
                    };
                    self.events.push(Event {
                        at: n.start,
                        id: n.id,
                        pitch: n.pitch,
                        velocity: n.velocity,
                        end,
                    });
                    self.events.push(Event {
                        at: end,
                        id: n.id,
                        pitch: n.pitch,
                        velocity: 0,
                        end,
                    });
                }
            }
            self.events
                .sort_by(|a, b| a.at.total_cmp(&b.at).then(a.velocity.cmp(&b.velocity)));
            self.revision = engine.song_revision;
            self.settings = Some(settings.clone());
            self.invalidate();
        }
        let (now, position, running) = engine.playback_cursor();
        // Offset is wall-clock milliseconds, even when practicing at a different speed.
        let reset =
            self.epoch != Some(engine.playback_epoch) || !running && !self.active.is_empty();
        if reset || (position - self.last_position).abs() > 0.000001 {
            self.stationary_since = now;
            self.last_position = position;
        }
        // A delayed onset already reached by the visual clock must still arrive while
        // the player is waiting. Do not advance into notes beyond that wait target.
        let offset = settings.audio_offset_ms / 1000.;
        let waiting_delay = if settings.practice_mode == "wait" && offset > 0. {
            (now - self.stationary_since).min(offset)
        } else {
            0.
        };
        let at = position - (offset - waiting_delay) * settings.speed;
        let mut out = PlaybackBatch {
            reset,
            ..Default::default()
        };
        if reset {
            self.epoch = Some(engine.playback_epoch);
            self.active.clear();
            self.counts.fill(0);
            self.attacks.fill(f64::NEG_INFINITY);
            self.cursor = self.events.partition_point(|event| event.at < at);
            if running {
                for i in 0..self.cursor {
                    let event = self.events[i];
                    if event.velocity > 0 && event.end > at {
                        self.start(event, now, at, settings.speed, &mut out.messages);
                    }
                }
            }
        }
        if !running {
            return out;
        }
        while self.cursor < self.events.len() && self.events[self.cursor].at <= at {
            let event = self.events[self.cursor];
            self.cursor += 1;
            if event.velocity > 0 {
                self.start(event, now, at, settings.speed, &mut out.messages);
            } else {
                self.release(event.id, &mut out.messages);
            }
        }
        // Waiting for a player must never sustain accompaniment indefinitely.
        let expired: Vec<_> = self
            .active
            .iter()
            .filter(|(_, n)| n.deadline <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.release(id, &mut out.messages);
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::Song;
    #[test]
    fn offsets_preserve_opening_notes_and_delayed_accompaniment_at_a_wait() {
        let mut e = engine("listen");
        let mut song = e.song.clone().unwrap();
        song.notes[0].start = 0.;
        song.notes[0].end = 0.2;
        e.load(song).unwrap();
        let mut s = e.settings.clone();
        s.audio_offset_ms = -500.;
        e.configure(s).unwrap();
        e.play().unwrap();
        let mut player = SongPlayback::default();
        assert!(player.update(&e).messages.contains(&[0x90, 48, 90]));
        e.tick(0.21);
        assert!(player.update(&e).messages.contains(&[0x80, 48, 0]));

        let mut e = engine("wait");
        let mut s = e.settings.clone();
        s.audio_offset_ms = 100.;
        e.configure(s).unwrap();
        e.play().unwrap();
        let mut player = SongPlayback::default();
        player.update(&e);
        e.tick(0.5);
        assert!(player.update(&e).messages.is_empty());
        assert_eq!(e.snapshot().waiting, vec![72]);
        e.tick(0.61);
        assert_eq!(player.update(&e).messages, vec![[0x90, 48, 90]]);
        e.tick(1.12);
        assert_eq!(player.update(&e).messages, vec![[0x80, 48, 0]]);
        assert_eq!(e.snapshot().waiting, vec![72]);
    }
    fn engine(mode: &str) -> PerformanceEngine {
        let mut e = PerformanceEngine::new(StageSettings::default());
        let song: Song = serde_json::from_value(serde_json::json!({
            "title":"test", "duration":3., "beats":[],
            "tracks":[{"id":0,"name":"Piano","channel":0,"percussion":false}],
            "notes":[{"id":0,"pitch":48,"start":1.,"end":1.5,"velocity":90,"track":0,"hand":"left"},
                {"id":1,"pitch":72,"start":1.,"end":1.5,"velocity":90,"track":0,"hand":"right"}]
        }))
        .unwrap();
        e.load(song).unwrap();
        let mut s = e.settings.clone();
        s.practice_mode = mode.into();
        s.practice_hand = "right".into();
        e.configure(s).unwrap();
        e.seek(0.5).unwrap();
        e.play().unwrap();
        e
    }
    #[test]
    fn repeated_overlapping_notes_keep_their_attacks_and_delayed_final_notes_finish() {
        let mut e = engine("listen");
        let mut song = e.song.clone().unwrap();
        song.notes.retain(|n| n.pitch == 48);
        song.notes[0].start = 0.5;
        song.notes[0].end = 2.;
        let mut second = song.notes[0].clone();
        second.id = 2;
        second.start = 1.;
        second.velocity = 110;
        song.notes.push(second);
        song.duration = 2.;
        e.load(song).unwrap();
        let mut s = e.settings.clone();
        s.audio_offset_ms = 200.;
        e.configure(s).unwrap();
        e.seek(0.1).unwrap();
        e.play().unwrap();
        let mut player = SongPlayback::default();
        player.update(&e);
        e.tick(0.61);
        assert_eq!(player.update(&e).messages, vec![[0x90, 48, 90]]);
        e.tick(1.11);
        assert_eq!(player.update(&e).messages, vec![[0x90, 48, 110]]);
        e.tick(1.91);
        assert!(e.snapshot().running);
        assert!(player.update(&e).messages.is_empty());
        e.tick(2.11);
        assert!(!e.snapshot().running);
        assert!(player.update(&e).reset);
    }
    #[test]
    fn accompaniment_plays_the_other_hand_once_and_releases_during_waits() {
        let mut e = engine("wait");
        let mut player = SongPlayback::default();
        assert!(player.update(&e).reset);
        e.tick(0.5);
        assert_eq!(player.update(&e).messages, vec![[0x90, 48, 90]]);
        assert_eq!(e.snapshot().waiting, vec![72]);
        e.tick(0.6);
        assert!(player.update(&e).messages.is_empty());
        e.tick(1.1);
        assert_eq!(player.update(&e).messages, vec![[0x80, 48, 0]]);
        e.tick(1.2);
        assert!(player.update(&e).messages.is_empty());
        e.pause();
        assert!(player.update(&e).reset);
        e.seek(0.5).unwrap();
        e.play().unwrap();
        player.update(&e);
        e.tick(1.7);
        assert_eq!(player.update(&e).messages, vec![[0x90, 48, 90]]);
    }
    #[test]
    fn listen_has_no_judgements_and_audio_offset_is_independent_of_input_latency() {
        let mut e = engine("listen");
        let mut s = e.settings.clone();
        s.audio_offset_ms = 100.;
        s.latency_ms = -200.;
        e.configure(s).unwrap();
        let mut player = SongPlayback::default();
        player.update(&e);
        e.tick(0.5);
        assert!(player.update(&e).messages.is_empty());
        e.tick(0.61);
        assert_eq!(player.update(&e).messages.len(), 2);
        e.input("keyboard", &[0x90, 80, 100], 0);
        assert_eq!(e.snapshot().score.wrong, 0);
        assert_eq!(e.snapshot().target_count, 0);
        e.pause();
        assert!(player.update(&e).reset);
        let mut s = e.settings.clone();
        s.playback_mode = "off".into();
        e.configure(s).unwrap();
        e.play().unwrap();
        assert!(player.update(&e).messages.is_empty());
    }
}
