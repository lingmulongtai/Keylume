// All unverified defaults live in resources/layout.json or this module.
// User verification overrides the addresses in the saved layout, never these constants.
pub const HEADER: [u8; 6] = [0xf0, 0, 0x20, 0x29, 2, 0x14];
pub const INQUIRY: [u8; 6] = [0xf0, 0x7e, 0x7f, 6, 1, 0xf7];
pub const DAW_ON: [u8; 3] = [0x9f, 0x0c, 0x7f];
pub const DAW_OFF: [u8; 3] = [0x9f, 0x0c, 0];
pub const PAD_MODE: u8 = 0x1d;
pub const DAW_DRUM: u8 = 0x54;
pub const MONO_CANDIDATES: [u8; 2] = [0xb3, 0x93];
pub const ENCODER_DOWN_CANDIDATES: [u8; 2] = [0x34, 0x44];
pub const OLED_TERMINATOR: u8 = 0xf7;
pub const VEGAS_ON_UNVERIFIED: u8 = 127;

/// Fixed MIDI input identities; LED output calibration must not rename controls.
pub fn surface_button(cc: u8) -> Option<String> {
    let id = match cc {
        37..=45 => return Some(format!("fbtn.{}", cc - 36)),
        51 => "btn.encoderUp",
        52 => "btn.encoderDown",
        63 => "btn.shift",
        74 => "btn.capture",
        75 => "btn.quantise",
        76 => "btn.metronome",
        77 => "btn.undo",
        102 => "btn.trackNext",
        103 => "btn.trackPrevious",
        104 => "btn.scene",
        105 => "btn.function",
        106 => "btn.padUp",
        107 => "btn.padDown",
        115 => "btn.play",
        116 => "btn.stop",
        117 => "btn.record",
        118 => "btn.loop",
        _ => return None,
    };
    Some(id.into())
}
