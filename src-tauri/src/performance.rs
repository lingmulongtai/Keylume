use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongNote {
    pub id: u32,
    pub pitch: u8,
    pub start: f64,
    pub end: f64,
    pub velocity: u8,
    pub track: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongTrack {
    pub id: u16,
    pub name: String,
    pub channel: u8,
    pub percussion: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub title: String,
    pub duration: f64,
    pub notes: Vec<SongNote>,
    pub tracks: Vec<SongTrack>,
    pub beats: Vec<f64>,
}
impl Song {
    pub fn validate(&self) -> Result<(), String> {
        let tracks: HashSet<_> = self.tracks.iter().map(|t| t.id).collect();
        let notes: HashSet<_> = self.notes.iter().map(|n| n.id).collect();
        if self.title.len() > 1024
            || self.notes.is_empty()
            || self.notes.len() > 50_000
            || self.tracks.len() > 256
            || tracks.len() != self.tracks.len()
            || notes.len() != self.notes.len()
            || !self.duration.is_finite()
            || !(0.01..=7200.).contains(&self.duration)
            || self
                .tracks
                .iter()
                .any(|t| t.channel > 15 || t.name.len() > 1024)
            || self.notes.iter().any(|n| {
                n.pitch > 127
                    || n.velocity == 0
                    || n.velocity > 127
                    || !tracks.contains(&n.track)
                    || !n.start.is_finite()
                    || !n.end.is_finite()
                    || n.start < 0.
                    || n.end <= n.start
                    || n.end > self.duration + 0.001
            })
            || self.beats.len() > 30_000
            || self
                .beats
                .iter()
                .any(|t| !t.is_finite() || *t < 0. || *t > self.duration + 0.001)
        {
            return Err("MIDIの音符・時間・トラックが範囲外です（最大50,000音・2時間）".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StageSettings {
    pub mode: String,
    pub practice_mode: String,
    pub speed: f64,
    pub look_ahead: f64,
    pub trail: f64,
    pub low: u8,
    pub high: u8,
    pub left: f64,
    pub right: f64,
    pub line_y: f64,
    pub style: String,
    pub color: String,
    pub labels: bool,
    pub guides: bool,
    pub particles: f64,
    pub latency_ms: f64,
    pub loop_enabled: bool,
    pub loop_start: f64,
    pub loop_end: f64,
    pub tracks: Vec<u16>,
}
impl Default for StageSettings {
    fn default() -> Self {
        Self {
            mode: "live".into(),
            practice_mode: "timing".into(),
            speed: 1.,
            look_ahead: 4.,
            trail: 6.,
            low: 36,
            high: 96,
            left: 0.05,
            right: 0.95,
            line_y: 0.82,
            style: "glow".into(),
            color: "#75c8fa".into(),
            labels: true,
            guides: false,
            particles: 0.6,
            latency_ms: 0.,
            loop_enabled: false,
            loop_start: 0.,
            loop_end: 8.,
            tracks: vec![],
        }
    }
}
impl StageSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !["live", "practice"].contains(&self.mode.as_str())
            || !["timing", "wait"].contains(&self.practice_mode.as_str())
            || !["clean", "glow", "particles", "rainbow"].contains(&self.style.as_str())
            || !(0.25..=2.).contains(&self.speed)
            || !(1.5..=12.).contains(&self.look_ahead)
            || !(2.0..=16.).contains(&self.trail)
            || self.high > 127
            || self.high.saturating_sub(self.low) < 12
            || !(0.0..=0.9).contains(&self.left)
            || !(0.1..=1.).contains(&self.right)
            || self.right - self.left < 0.1
            || !(0.25..=0.95).contains(&self.line_y)
            || !(0.0..=1.).contains(&self.particles)
            || !(-500.0..=500.).contains(&self.latency_ms)
            || !(0.0..=7200.).contains(&self.loop_start)
            || !(0.1..=7200.).contains(&self.loop_end)
            || self.loop_end - self.loop_start < 0.1
            || self.color.len() != 7
            || !self.color.starts_with('#')
            || !self.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            || self.tracks.len() > 256
        {
            return Err("演奏画面の設定値が範囲外です".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveNote {
    pub id: u64,
    pub pitch: u8,
    pub velocity: u8,
    pub start: f64,
    pub end: Option<f64>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub perfect: u32,
    pub good: u32,
    pub late: u32,
    pub miss: u32,
    pub wrong: u32,
    pub combo: u32,
    pub best: u32,
    pub points: u32,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Judgement {
    pub pitch: u8,
    pub result: String,
    pub offset_ms: f64,
    pub at: f64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageSnapshot {
    pub clock: f64,
    pub position: f64,
    pub running: bool,
    pub waiting: Vec<u8>,
    pub next_stop: Option<f64>,
    pub score: Score,
    pub live: Vec<LiveNote>,
    pub held: Vec<u8>,
    pub judgements: Vec<Judgement>,
    pub settings: StageSettings,
    pub song_revision: u64,
    pub title: String,
    pub duration: f64,
    pub loop_count: u32,
    pub target_count: usize,
}
struct Target {
    pitch: u8,
    start: f64,
    status: u8,
}
pub struct PerformanceEngine {
    pub settings: StageSettings,
    pub song: Option<Song>,
    pub song_revision: u64,
    clock: f64,
    position: f64,
    running: bool,
    score: Score,
    targets: Vec<Target>,
    next_miss: usize,
    live: VecDeque<LiveNote>,
    pressed: HashMap<(String, u8, u8), u64>,
    sustained: HashMap<(String, u8), Vec<u64>>,
    pedals: HashSet<(String, u8)>,
    next_id: u64,
    judgements: VecDeque<Judgement>,
    loop_count: u32,
}
impl PerformanceEngine {
    pub fn new(settings: StageSettings) -> Self {
        Self {
            settings,
            song: None,
            song_revision: 0,
            clock: 0.,
            position: 0.,
            running: false,
            score: Score::default(),
            targets: vec![],
            next_miss: 0,
            live: VecDeque::new(),
            pressed: HashMap::new(),
            sustained: HashMap::new(),
            pedals: HashSet::new(),
            next_id: 0,
            judgements: VecDeque::new(),
            loop_count: 0,
        }
    }
    pub fn load(&mut self, mut song: Song) -> Result<(), String> {
        song.validate()?;
        song.notes
            .sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
        self.settings.tracks = song
            .tracks
            .iter()
            .filter(|t| !t.percussion)
            .map(|t| t.id)
            .collect();
        self.settings.loop_enabled = false;
        self.settings.loop_start = 0.;
        self.settings.loop_end = song.duration.max(0.1);
        self.settings.mode = "practice".into();
        self.song = Some(song);
        self.song_revision += 1;
        self.reset_attempt(0.);
        Ok(())
    }
    pub fn configure(&mut self, settings: StageSettings) -> Result<(), String> {
        settings.validate()?;
        let restart = self.settings.tracks != settings.tracks
            || self.settings.practice_mode != settings.practice_mode
            || self.settings.loop_enabled != settings.loop_enabled
            || self.settings.loop_start != settings.loop_start
            || self.settings.loop_end != settings.loop_end;
        let mode_changed = self.settings.mode != settings.mode;
        self.settings = settings;
        if restart {
            self.reset_attempt(self.position.max(0.));
        }
        if mode_changed {
            self.running = false;
        }
        Ok(())
    }
    fn rebuild_targets(&mut self, from: f64) {
        self.targets.clear();
        let mut seen = HashSet::new();
        if let Some(song) = &self.song {
            for n in &song.notes {
                if !self.settings.tracks.contains(&n.track)
                    || (self.settings.loop_enabled
                        && (n.start < self.settings.loop_start
                            || n.start >= self.settings.loop_end))
                {
                    continue;
                }
                if seen.insert(((n.start * 1000.).round() as u64, n.pitch)) {
                    self.targets.push(Target {
                        pitch: n.pitch,
                        start: n.start,
                        status: if n.start + 0.001 < from { 5 } else { 0 },
                    });
                }
            }
        }
        self.next_miss = self.targets.partition_point(|n| n.status != 0);
    }
    fn reset_attempt(&mut self, position: f64) {
        self.running = false;
        self.position = position;
        self.score = Score::default();
        self.judgements.clear();
        self.loop_count = 0;
        self.rebuild_targets(position);
    }
    pub fn seek(&mut self, position: f64) -> Result<(), String> {
        let duration = self
            .song
            .as_ref()
            .ok_or("MIDIを読み込んでください")?
            .duration;
        if !position.is_finite() || !(0.0..=duration).contains(&position) {
            return Err("移動先が範囲外です".into());
        }
        self.reset_attempt(position);
        Ok(())
    }
    pub fn play(&mut self) -> Result<(), String> {
        if self.song.is_none() || self.targets.is_empty() {
            return Err("演奏するMIDIトラックを選択してください".into());
        }
        if self.settings.mode != "practice" {
            return Err("MIDI練習モードを選択してください".into());
        }
        let begin = if self.settings.loop_enabled {
            self.settings.loop_start
        } else {
            0.
        };
        let end = self.end();
        if self.position >= end || self.position < begin {
            self.reset_attempt(begin);
        }
        if (self.position - begin).abs() < 0.001 {
            self.position = begin - self.settings.look_ahead.min(3.);
        }
        self.running = true;
        Ok(())
    }
    pub fn pause(&mut self) {
        self.running = false;
    }
    pub fn stop(&mut self) {
        let begin = if self.settings.loop_enabled {
            self.settings.loop_start
        } else {
            0.
        };
        self.reset_attempt(begin);
    }
    fn end(&self) -> f64 {
        self.song
            .as_ref()
            .map(|s| {
                if self.settings.loop_enabled {
                    self.settings.loop_end.min(s.duration)
                } else {
                    s.duration
                }
            })
            .unwrap_or(0.)
    }
    fn next_stop(&self) -> Option<f64> {
        if self.settings.practice_mode == "wait" {
            self.targets[self.next_miss..]
                .iter()
                .find(|n| n.status == 0)
                .map(|n| n.start)
        } else {
            None
        }
    }
    pub fn tick(&mut self, now: f64) {
        let dt = (now - self.clock).max(0.);
        self.clock = now;
        if self.running {
            if dt > 1. {
                self.running = false;
            } else {
                let proposed = self.position + dt * self.settings.speed;
                self.position = self
                    .next_stop()
                    .map_or(proposed, |at| proposed.min(at.max(self.position)));
                if self.settings.practice_mode == "timing" {
                    while self.next_miss < self.targets.len()
                        && self.targets[self.next_miss].start
                            < self.position - 0.25 * self.settings.speed
                    {
                        if self.targets[self.next_miss].status == 0 {
                            let pitch = self.targets[self.next_miss].pitch;
                            self.targets[self.next_miss].status = 4;
                            self.record(pitch, "miss", 0.);
                        }
                        self.next_miss += 1;
                    }
                }
                if self.position
                    >= self.end()
                        + if self.settings.practice_mode == "timing" {
                            0.25 * self.settings.speed
                        } else {
                            0.
                        }
                {
                    if self.settings.loop_enabled {
                        self.position = self.settings.loop_start - 1.;
                        self.loop_count += 1;
                        self.rebuild_targets(self.settings.loop_start);
                    } else {
                        self.running = false;
                        self.position = self.end();
                    }
                }
            }
        }
        let cutoff = now - self.settings.trail - 1.;
        self.live.retain(|n| n.end.is_none_or(|end| end >= cutoff));
        while self.live.len() > 2048 {
            self.live.pop_front();
        }
        self.judgements.retain(|j| now - j.at < 1.5);
    }
    fn record(&mut self, pitch: u8, result: &str, offset: f64) {
        match result {
            "perfect" => {
                self.score.perfect += 1;
                self.score.points += 100;
            }
            "good" => {
                self.score.good += 1;
                self.score.points += 70;
            }
            "late" => {
                self.score.late += 1;
                self.score.points += 40;
            }
            "miss" => self.score.miss += 1,
            _ => self.score.wrong += 1,
        }
        if result == "miss" || result == "wrong" {
            self.score.combo = 0;
        } else {
            self.score.combo += 1;
            self.score.best = self.score.best.max(self.score.combo);
        }
        if self.judgements.len() >= 64 {
            self.judgements.pop_front();
        }
        self.judgements.push_back(Judgement {
            pitch,
            result: result.into(),
            offset_ms: offset * 1000.,
            at: self.clock,
        });
    }
    fn hit(&mut self, pitch: u8) {
        if !self.running || self.settings.mode != "practice" {
            return;
        }
        let at = self.position
            - if self.settings.practice_mode == "timing" {
                self.settings.latency_ms / 1000. * self.settings.speed
            } else {
                0.
            };
        let lower = self
            .targets
            .partition_point(|n| n.start < at - 0.25 * self.settings.speed);
        let upper = self
            .targets
            .partition_point(|n| n.start <= at + 0.25 * self.settings.speed);
        let candidate = self.targets[lower..upper]
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.status == 0
                    && n.pitch == pitch
                    && ((at - n.start) / self.settings.speed).abs() <= 0.25
            })
            .min_by(|(_, a), (_, b)| (at - a.start).abs().total_cmp(&(at - b.start).abs()))
            .map(|(i, _)| i + lower);
        if let Some(i) = candidate {
            let offset = (at - self.targets[i].start) / self.settings.speed;
            let (status, label) = if offset.abs() <= 0.08 {
                (1, "perfect")
            } else if offset.abs() <= 0.16 {
                (2, "good")
            } else {
                (3, "late")
            };
            self.targets[i].status = status;
            self.record(pitch, label, offset);
            while self.next_miss < self.targets.len() && self.targets[self.next_miss].status != 0 {
                self.next_miss += 1;
            }
        } else if self.position >= 0. {
            self.record(pitch, "wrong", 0.);
        }
    }
    fn end_note(&mut self, id: u64) {
        if let Some(note) = self.live.iter_mut().find(|n| n.id == id) {
            note.end = Some(self.clock);
        }
    }
    pub fn release_source(&mut self, source: &str) {
        let ids: Vec<_> = self
            .pressed
            .iter()
            .filter(|((s, _, _), _)| s == source)
            .map(|(_, id)| *id)
            .chain(
                self.sustained
                    .iter()
                    .filter(|((s, _), _)| s == source)
                    .flat_map(|(_, ids)| ids.iter().copied()),
            )
            .collect();
        for id in ids {
            self.end_note(id);
        }
        self.pressed.retain(|(s, _, _), _| s != source);
        self.sustained.retain(|(s, _), _| s != source);
        self.pedals.retain(|(s, _)| s != source);
    }
    pub fn input(&mut self, source: &str, b: &[u8], octave: i8) {
        if !["keyboard", "screen"].contains(&source) || b.len() != 3 || b[1] > 127 || b[2] > 127 {
            return;
        }
        let ch = b[0] & 15;
        let key = (source.to_string(), ch, b[1]);
        let pedal = (source.to_string(), ch);
        match b[0] & 0xf0 {
            0x90 if b[2] > 0 => {
                let pitch = b[1] as i16 + 12 * octave as i16;
                if !(0..=127).contains(&pitch) {
                    return;
                }
                if let Some(id) = self.pressed.remove(&key) {
                    self.end_note(id);
                }
                self.next_id += 1;
                self.pressed.insert(key, self.next_id);
                self.live.push_back(LiveNote {
                    id: self.next_id,
                    pitch: pitch as u8,
                    velocity: b[2],
                    start: self.clock,
                    end: None,
                });
                self.hit(pitch as u8);
            }
            0x80 | 0x90 => {
                if let Some(id) = self.pressed.remove(&key) {
                    if self.pedals.contains(&pedal) {
                        self.sustained.entry(pedal).or_default().push(id);
                    } else {
                        self.end_note(id);
                    }
                }
            }
            0xb0 if b[1] == 64 => {
                if b[2] >= 64 {
                    self.pedals.insert(pedal);
                } else {
                    self.pedals.remove(&pedal);
                    if let Some(ids) = self.sustained.remove(&pedal) {
                        for id in ids {
                            self.end_note(id);
                        }
                    }
                }
            }
            0xb0 if [120, 121, 123].contains(&b[1]) => self.release_source(source),
            _ => {}
        }
    }
    pub fn snapshot(&self) -> StageSnapshot {
        let next_stop = self.next_stop();
        let waiting = if self.running && next_stop.is_some_and(|t| t <= self.position + 0.001) {
            self.targets
                .iter()
                .filter(|n| n.status == 0 && (n.start - self.position).abs() < 0.025)
                .map(|n| n.pitch)
                .collect()
        } else {
            vec![]
        };
        let mut held: Vec<_> = self
            .live
            .iter()
            .filter(|n| n.end.is_none())
            .map(|n| n.pitch)
            .collect();
        held.sort();
        held.dedup();
        StageSnapshot {
            clock: self.clock,
            position: self.position,
            running: self.running,
            waiting,
            next_stop,
            score: self.score.clone(),
            live: self.live.iter().cloned().collect(),
            held,
            judgements: self.judgements.iter().cloned().collect(),
            settings: self.settings.clone(),
            song_revision: self.song_revision,
            title: self
                .song
                .as_ref()
                .map(|s| s.title.clone())
                .unwrap_or_default(),
            duration: self.song.as_ref().map(|s| s.duration).unwrap_or(0.),
            loop_count: self.loop_count,
            target_count: self.targets.len(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn song() -> Song {
        Song {
            title: "test".into(),
            duration: 3.,
            tracks: vec![SongTrack {
                id: 0,
                name: "Piano".into(),
                channel: 0,
                percussion: false,
            }],
            notes: vec![
                SongNote {
                    id: 0,
                    pitch: 60,
                    start: 1.,
                    end: 1.5,
                    velocity: 100,
                    track: 0,
                },
                SongNote {
                    id: 1,
                    pitch: 64,
                    start: 1.,
                    end: 1.5,
                    velocity: 100,
                    track: 0,
                },
                SongNote {
                    id: 2,
                    pitch: 67,
                    start: 2.,
                    end: 2.5,
                    velocity: 100,
                    track: 0,
                },
            ],
            beats: vec![0., 0.5, 1., 1.5, 2., 2.5, 3.],
        }
    }
    fn advance(e: &mut PerformanceEngine, to: f64) {
        while e.clock + 0.01 < to {
            e.tick(e.clock + 0.01);
        }
        e.tick(to);
    }
    #[test]
    fn timing_grades_chords_misses_and_wrong_notes() {
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.load(song()).unwrap();
        e.seek(0.5).unwrap();
        e.play().unwrap();
        advance(&mut e, 0.5);
        e.input("keyboard", &[0x90, 60, 100], 0);
        e.input("keyboard", &[0x90, 64, 100], 0);
        assert_eq!(e.score.perfect, 2);
        e.input("keyboard", &[0x90, 65, 100], 0);
        assert_eq!(e.score.wrong, 1);
        advance(&mut e, 2.);
        assert_eq!(e.score.miss, 1);
        assert_eq!(e.score.best, 2);
    }
    #[test]
    fn wait_mode_holds_a_chord_until_both_keys_are_played() {
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.load(song()).unwrap();
        let mut settings = e.settings.clone();
        settings.practice_mode = "wait".into();
        settings.latency_ms = 500.;
        e.configure(settings).unwrap();
        e.seek(0.5).unwrap();
        e.play().unwrap();
        advance(&mut e, 1.);
        assert_eq!(e.position, 1.);
        e.input("keyboard", &[0x90, 60, 100], 0);
        advance(&mut e, 1.1);
        assert_eq!(e.position, 1.);
        e.input("keyboard", &[0x90, 64, 100], 0);
        advance(&mut e, 1.2);
        assert!(e.position > 1.);
    }
    #[test]
    fn sustain_and_sources_keep_live_trails_until_the_correct_release() {
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.tick(0.1);
        e.input("keyboard", &[0xb0, 64, 127], 0);
        e.input("keyboard", &[0x90, 60, 100], 1);
        e.input("screen", &[0x90, 60, 100], 0);
        e.input("keyboard", &[0x80, 60, 0], 1);
        assert_eq!(e.snapshot().held, vec![60, 72]);
        e.release_source("screen");
        assert_eq!(e.snapshot().held, vec![72]);
        e.input("keyboard", &[0xb0, 64, 0], 0);
        assert!(e.snapshot().held.is_empty());
    }
    #[test]
    fn seek_loop_speed_and_sleep_do_not_create_phantom_misses() {
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.load(song()).unwrap();
        let mut settings = e.settings.clone();
        settings.speed = 0.5;
        settings.loop_enabled = true;
        settings.loop_start = 1.;
        settings.loop_end = 2.8;
        e.configure(settings).unwrap();
        e.seek(1.2).unwrap();
        e.play().unwrap();
        advance(&mut e, 0.2);
        assert!((e.position - 1.3).abs() < 0.001);
        e.pause();
        advance(&mut e, 0.4);
        assert!((e.position - 1.3).abs() < 0.001);
        e.play().unwrap();
        advance(&mut e, 4.);
        assert_eq!(e.loop_count, 1);
        let score = e.score.miss;
        e.tick(20.);
        assert!(!e.running);
        assert_eq!(e.score.miss, score);
    }
    #[test]
    fn final_short_note_keeps_its_late_hit_window_and_counts_misses() {
        let mut s = song();
        s.notes = vec![SongNote {
            id: 0,
            pitch: 60,
            start: 0.,
            end: 0.02,
            velocity: 100,
            track: 0,
        }];
        s.duration = 0.02;
        s.beats = vec![0.];
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.load(s).unwrap();
        e.settings.validate().unwrap();
        e.play().unwrap();
        advance(&mut e, 3.1);
        assert!(e.running);
        e.input("keyboard", &[0x90, 60, 100], 0);
        assert_eq!(e.score.good, 1);
        e.stop();
        e.play().unwrap();
        advance(&mut e, 6.5);
        assert_eq!(e.score.miss, 1);
        assert!(!e.running);
    }
    #[test]
    fn imported_unison_notes_need_only_one_key_and_invalid_songs_are_rejected() {
        let mut song = song();
        let mut duplicate = song.notes[0].clone();
        duplicate.id = 3;
        song.notes.push(duplicate);
        let mut e = PerformanceEngine::new(StageSettings::default());
        e.load(song.clone()).unwrap();
        assert_eq!(e.targets.len(), 3);
        song.notes[0].end = f64::NAN;
        assert!(song.validate().is_err());
        let mut settings = StageSettings::default();
        settings.left = 0.9;
        settings.right = 0.5;
        assert!(settings.validate().is_err());
        assert_eq!(StageSettings::default().practice_mode, "timing");
    }
}
