use keylume_lib::piano::{sound_font, PianoSynth};
use rustysynth::SoundFont;
use std::{fs::File, sync::Arc};
fn main() {
    let mut fonts = vec![("upright", sound_font().unwrap())];
    let path = std::env::args()
        .nth(1)
        .expect("supply local Grand Piano SoundFont");
    fonts.push((
        "grand",
        Arc::new(SoundFont::new(&mut File::open(path).unwrap()).unwrap()),
    ));
    for (name, font) in fonts {
        for velocity in [35, 80, 110, 127] {
            let mut synth = PianoSynth::new(&font, 48000).unwrap();
            for pitch in [48, 55, 60, 64, 67, 72] {
                synth.midi(false, [0x90, pitch, velocity]);
            }
            let mut left = vec![0.; 48000];
            let mut right = left.clone();
            synth.render(&mut left, &mut right);
            let peak = left
                .iter()
                .chain(&right)
                .map(|x| x.abs())
                .fold(0f32, f32::max);
            let rms = (left.iter().chain(&right).map(|x| x * x).sum::<f32>() / 96000.).sqrt();
            println!(
                "{name} velocity={velocity} peak={peak:.5} rms={rms:.5} peak_db={:.1} rms_db={:.1}",
                20. * peak.log10(),
                20. * rms.log10()
            );
        }
    }
}
