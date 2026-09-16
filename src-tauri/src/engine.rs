use crate::model::{DeviceLayout, Layer, Post, Preset};
use std::collections::HashSet;
use std::f32::consts::TAU;
pub type Color = [f32; 3];
#[derive(Clone)]
pub struct Hit {
    pub x: f32,
    pub y: f32,
    pub at: f32,
    pub velocity: f32,
    pub led: Option<String>,
    pub note: u8,
}
pub struct Engine {
    pub hits: Vec<Hit>,
    pub held: HashSet<u8>,
    pub bands: [f32; 8],
    pub peaks: [f32; 8],
    pub bpm: f32,
    pub time: f32,
    previous: Vec<Color>,
    transition_at: f32,
    clock_last: Option<f32>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            hits: vec![],
            held: HashSet::new(),
            bands: [0.; 8],
            peaks: [0.; 8],
            bpm: 120.,
            time: 0.,
            previous: vec![],
            transition_at: -1.,
            clock_last: None,
        }
    }
}
impl Engine {
    pub fn hit(&mut self, hit: Hit) {
        if self.hits.len() >= 64 {
            self.hits.remove(0);
        }
        self.hits.push(hit);
    }
    pub fn clock(&mut self) {
        if let Some(last) = self.clock_last {
            let delta = self.time - last;
            if (0.005..0.2).contains(&delta) {
                self.bpm = self.bpm * 0.8 + 60. / (delta * 24.) * 0.2;
            }
        }
        self.clock_last = Some(self.time);
    }
    pub fn transition(&mut self, frame: Vec<Color>) {
        self.previous = frame;
        self.transition_at = self.time;
    }
    pub fn render(
        &mut self,
        p: &Preset,
        layout: &DeviceLayout,
        brightness: f32,
    ) -> (Vec<Color>, Vec<[u8; 3]>) {
        self.hits.retain(|h| self.time - h.at < 10.);
        let mut colors = vec![[0.; 3]; layout.leds.len()];
        for layer in p.layers.iter().rev().filter(|l| l.enabled) {
            for (i, led) in layout.leds.iter().enumerate() {
                if led.kind == "none" || !layer.zone.contains(led) {
                    continue;
                }
                if layer.effect == "paint"
                    && layer
                        .params
                        .get("colorsByLed")
                        .and_then(|v| v.get(&led.id))
                        .is_none()
                {
                    continue;
                }
                let x = led.pos.x / layout.canvas.w;
                let y = led.pos.y / layout.canvas.h;
                let c = self.effect(layer, x, y, i, &led.id);
                colors[i] = blend(colors[i], c, layer.opacity, &layer.blend);
            }
        }
        let transition = ((self.time - self.transition_at) / 0.5).clamp(0., 1.);
        if transition < 1. && self.previous.len() == colors.len() {
            for (c, old) in colors.iter_mut().zip(&self.previous) {
                *c = mix(*old, *c, transition);
            }
        }
        let quantized = colors
            .iter()
            .zip(&layout.leds)
            .map(|(c, l)| {
                if l.kind == "none" {
                    [0; 3]
                } else {
                    quantize(*c, &p.post, brightness, l.kind == "mono")
                }
            })
            .collect();
        (colors, quantized)
    }
    fn effect(&self, l: &Layer, x: f32, y: f32, index: usize, id: &str) -> Color {
        let speed = l.number("speed", 0.6).clamp(0., 10.);
        let t = self.time * speed;
        let color = hex(l.text("color", "#43ffc2"));
        let colors: Vec<Color> = l
            .params
            .get("colors")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(hex)).collect())
            .filter(|a: &Vec<Color>| !a.is_empty())
            .unwrap_or_else(|| vec![color, [0.23, 0.57, 0.9], [0.62, 0.31, 0.95]]);
        let coord = match l.text("direction", "right") {
            "left" => 1. - x,
            "down" => y,
            "out" => ((x - 0.5).powi(2) + (y - 0.4).powi(2)).sqrt(),
            "in" => 1. - ((x - 0.5).powi(2) + (y - 0.4).powi(2)).sqrt(),
            _ => x,
        };
        match l.effect.as_str() {
            "static" => color,
            "paint" => l
                .params
                .get("colorsByLed")
                .and_then(|v| v.get(id))
                .and_then(|v| v.as_str())
                .map(hex)
                .unwrap_or([0.; 3]),
            "gradient" => {
                let a = l.number("angle", 0.).to_radians();
                ramp(
                    &colors,
                    ((x - 0.5) * a.cos() + (y - 0.5) * a.sin() + 0.5).clamp(0., 1.),
                )
            }
            "breathing" => scale(
                ramp(
                    &colors,
                    (self.time / l.number("period", 6.).max(0.2) / 3.).fract(),
                ),
                ((self.time / l.number("period", 6.).max(0.2) * TAU).sin() * 0.5 + 0.5).powf(1.5),
            ),
            "spectrum_cycle" => hsv(
                (self.time / l.number("period", 8.).max(0.2)).fract(),
                1.,
                1.,
            ),
            "wave" => hsv(
                (coord / l.number("wavelength", 0.8).max(0.1) - t * 0.1).rem_euclid(1.),
                0.85,
                1.,
            ),
            "aurora" => {
                let n =
                    ((x * 6. + t * 0.6).sin() + (x * 11. - y * 5. - t * 0.4).cos()) * 0.25 + 0.5;
                scale(ramp(&colors, n), 0.5 + 0.5 * (x * 8. + t).sin().abs())
            }
            "fire" => {
                let n =
                    ((x * 31. + t * 2.).sin() + (x * 57. - t * 4. + y * 17.).sin()) * 0.2 + 0.55;
                scale(
                    ramp(&[[0.8, 0.01, 0.0], [1., 0.16, 0.01], [1., 0.75, 0.12]], n),
                    l.number("intensity", 1.).clamp(0., 2.),
                )
            }
            "starlight" => {
                let phase = ((index as f32 * 12.9898).sin() * 43758.547).fract().abs();
                let v = (t * 0.3 + phase).fract();
                let density = l.number("density", 0.3).clamp(0.02, 1.);
                scale(
                    color,
                    ((v - (1. - density)) / density * std::f32::consts::PI)
                        .sin()
                        .max(0.)
                        .powi(4),
                )
            }
            "ripple" | "reactive" => {
                let mut result = [0.; 3];
                let decay = l.number("decay", 2.).clamp(0.1, 8.);
                for h in &self.hits {
                    let age = self.time - h.at;
                    let distance = ((x - h.x).powi(2) + (y - h.y).powi(2)).sqrt();
                    let v = if l.effect == "ripple" {
                        (1. - (distance - age * 0.3 * speed.max(0.1)).abs() / 0.09).max(0.)
                    } else if h.led.as_deref() == Some(id)
                        || (h.led.is_none() && (x - h.x).abs() < 0.055)
                    {
                        1.
                    } else {
                        0.
                    };
                    result = blend(
                        result,
                        color,
                        v * (1. - age / decay).max(0.) * h.velocity,
                        "add",
                    );
                }
                result
            }
            "note_map" => {
                let mut out = [0.; 3];
                for note in &self.held {
                    let nx = 0.22 + ((*note as f32 - 36.) / 60.).clamp(0., 1.) * 0.74;
                    if (x - nx).abs() < 0.065 {
                        out = blend(out, hsv((*note % 12) as f32 / 12., 0.8, 1.), 1., "add");
                    }
                }
                out
            }
            "chord_color" => {
                if self.held.is_empty() {
                    [0.; 3]
                } else {
                    let mut out = [0.; 3];
                    for note in &self.held {
                        let c = hsv((*note % 12) as f32 / 12., 0.8, 1.);
                        for i in 0..3 {
                            out[i] += c[i] / self.held.len() as f32;
                        }
                    }
                    out
                }
            }
            "audio_spectrum" => {
                let band = if id.starts_with("pad.") {
                    id.rsplit('.')
                        .next()
                        .and_then(|n| n.parse::<usize>().ok())
                        .unwrap_or(1)
                        .saturating_sub(1)
                        .min(7)
                } else {
                    ((x - 0.64) * 24.).clamp(0., 7.) as usize
                };
                let level = if id.starts_with("pad.top") {
                    self.peaks[band]
                } else {
                    self.bands[band]
                };
                scale(
                    hsv(0.42 - level * 0.4, 0.85, 1.),
                    level * l.number("sensitivity", 1.),
                )
            }
            "audio_pulse" => scale(
                color,
                ((self.bands[0] + self.bands[1]) * 0.5 * l.number("sensitivity", 1.)
                    - l.number("threshold", 0.05))
                .max(0.),
            ),
            "tempo_pulse" | "metronome" => {
                let bpm = if l.text("source", "fixed") == "midi" {
                    self.bpm
                } else {
                    l.number("bpm", 120.)
                };
                let beat = self.time * bpm / 60.;
                let v = (1. - beat.fract() * 4.).max(0.);
                scale(
                    color,
                    if l.effect == "metronome"
                        && beat as u32 % l.number("beats", 4.).max(1.) as u32 != 0
                    {
                        v * 0.25
                    } else {
                        v
                    },
                )
            }
            "hardware_fx" => {
                static PALETTE: std::sync::OnceLock<Vec<[u8; 3]>> = std::sync::OnceLock::new();
                let palette = PALETTE.get_or_init(|| {
                    serde_json::from_str(include_str!("../../resources/palette.json"))
                        .unwrap_or_default()
                });
                let c = palette
                    .get(l.number("palette", 76.).clamp(0., 127.) as usize)
                    .copied()
                    .unwrap_or([64, 255, 128])
                    .map(|v| v as f32 / 255.);
                let beat = self.time * self.bpm / 60.;
                scale(
                    c,
                    match l.text("mode", "pulse") {
                        "flash" => {
                            if beat.fract() < 0.5 {
                                1.
                            } else {
                                0.
                            }
                        }
                        "pulse" => (beat * std::f32::consts::PI).sin() * 0.5 + 0.5,
                        _ => 1.,
                    },
                )
            }
            _ => [0.; 3],
        }
    }
}
pub fn hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if s.len() != 6 || !s.is_ascii() {
        return [0.25, 1., 0.75];
    }
    std::array::from_fn(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0) as f32 / 255.)
}
pub fn scale(c: Color, v: f32) -> Color {
    c.map(|c| (c * v).clamp(0., 1.))
}
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t)
}
pub fn ramp(c: &[Color], v: f32) -> Color {
    let p = v.clamp(0., 1.) * (c.len() - 1) as f32;
    let i = p as usize;
    mix(c[i], c[(i + 1).min(c.len() - 1)], p.fract())
}
pub fn hsv(h: f32, s: f32, v: f32) -> Color {
    [0., 4., 2.].map(|n| {
        let k = (n + h * 6.).rem_euclid(6.);
        v * (1. - s * k.min(4. - k).clamp(0., 1.))
    })
}
pub fn blend(a: Color, b: Color, alpha: f32, mode: &str) -> Color {
    std::array::from_fn(|i| {
        let target = match mode {
            "add" => (a[i] + b[i]).min(1.),
            "multiply" => a[i] * b[i],
            "screen" => 1. - (1. - a[i]) * (1. - b[i]),
            "max" => a[i].max(b[i]),
            _ => b[i],
        };
        (a[i] + (target - a[i]) * alpha).clamp(0., 1.)
    })
}
pub fn quantize(c: Color, p: &Post, master: f32, mono: bool) -> [u8; 3] {
    let c = scale(c, p.brightness * master);
    let lum = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
    let mut c = c.map(|v| (lum + (v - lum) * p.saturation).clamp(0., 1.));
    let warm = ((6500. - p.temperature_k) / 5500.).clamp(0., 1.);
    let cool = ((p.temperature_k - 6500.) / 5500.).clamp(0., 1.);
    c[0] *= 1. - cool * 0.3;
    c[1] *= 1. - warm * 0.15;
    c[2] *= 1. - warm * 0.65;
    c = c.map(|v| v.clamp(0., 1.).powf(p.gamma));
    if mono {
        c = [c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722; 3];
    }
    c.map(|v| (v * 127.).round() as u8)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::builtin_presets;
    #[test]
    fn blend_modes_obey_alpha() {
        for mode in ["normal", "add", "multiply", "screen", "max"] {
            assert_eq!(blend([0.2; 3], [0.7; 3], 0., mode), [0.2; 3]);
        }
        assert_eq!(blend([0.2; 3], [0.7; 3], 1., "max"), [0.7; 3]);
        assert!((blend([0.2; 3], [0.7; 3], 1., "screen")[0] - 0.76).abs() < 0.001);
    }
    #[test]
    fn quantization_gamma_and_mono() {
        let p = Post {
            brightness: 1.,
            saturation: 1.,
            temperature_k: 6500.,
            gamma: 2.2,
        };
        assert_eq!(quantize([1.; 3], &p, 1., false), [127; 3]);
        assert_eq!(quantize([1.; 3], &p, 0., false), [0; 3]);
        assert_eq!(quantize([0.5; 3], &p, 1., false), [28; 3]);
        let mono = quantize([1., 0., 0.], &p, 1., true);
        assert_eq!(mono[0], mono[1]);
        assert!(mono[0] < 127);
    }
    #[test]
    fn all_effects_produce_bounded_frames() {
        let mut e = Engine::default();
        let layout = DeviceLayout::default();
        let mut p = builtin_presets().remove(0);
        for id in crate::model::EFFECTS {
            p.layers[0].effect = id.to_string();
            for t in [0., 0.5, 10000.] {
                e.time = t;
                let (c, q) = e.render(&p, &layout, 1.);
                assert!(c
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
                assert!(q.iter().flatten().all(|v| *v <= 127));
            }
        }
    }
    #[test]
    fn zone_does_not_bleed() {
        let mut p = builtin_presets().remove(0);
        p.layers[0].zone = crate::model::Zone::Named("pads.top".into());
        let (_, q) = Engine::default().render(&p, &DeviceLayout::default(), 1.);
        assert!(q[8..].iter().all(|v| *v == [0; 3]));
    }
}
