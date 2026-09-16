use super::constants::*;
use crate::model::LedDef;
pub fn sysex(body: &[u8]) -> Result<Vec<u8>, String> {
    if body.iter().any(|&b| b > 127) {
        return Err("SysEx data must be 7-bit".into());
    }
    let mut out = HEADER.to_vec();
    out.extend_from_slice(body);
    out.push(0xf7);
    Ok(out)
}
pub fn rgb(led: &LedDef, color: [u8; 3], drum: bool) -> Result<Vec<u8>, String> {
    let id = if drum && led.group == "pads" {
        led.address.drum_note
    } else {
        led.address.sysex_id
    };
    sysex(&[
        1,
        if led.group == "pads" { 0x43 } else { 0x53 },
        id.ok_or("Missing RGB address")?,
        color[0],
        color[1],
        color[2],
    ])
}
pub fn mono(led: &LedDef, level: u8) -> Result<Vec<u8>, String> {
    if level > 127 {
        return Err("Brightness must be 7-bit".into());
    }
    Ok(vec![
        led.address.mono_status.unwrap_or(0xb3),
        led.address.cc.ok_or("Missing CC")?,
        level,
    ])
}
pub fn palette(led: &LedDef, color: u8, mode: u8, drum: bool) -> Result<Vec<u8>, String> {
    if color > 127 || mode > 2 {
        return Err("Invalid palette command".into());
    }
    if led.group == "pads" {
        Ok(vec![
            if drum { 0x99 } else { 0x90 } + mode,
            if drum {
                led.address.drum_note
            } else {
                led.address.daw_note
            }
            .ok_or("Missing note")?,
            color,
        ])
    } else {
        Ok(vec![
            0xb0 + mode,
            led.address.cc.ok_or("Missing CC")?,
            color,
        ])
    }
}
pub fn bitmap(bits: &[u8], target: u8) -> Result<Vec<u8>, String> {
    if bits.len() != 128 * 64 || bits.iter().any(|&v| v > 1) {
        return Err("Expected 128x64 1-bit pixels".into());
    }
    let mut data = vec![9, target];
    for row in bits.chunks_exact(128) {
        for chunk in row.chunks(7) {
            data.push(
                chunk
                    .iter()
                    .enumerate()
                    .fold(0, |v, (i, b)| v | (b << (6 - i))),
            );
        }
    }
    sysex(&data)
}
pub fn display_text(text: &str, target: u8) -> Result<Vec<Vec<u8>>, String> {
    let ascii: Vec<u8> = text
        .bytes()
        .filter(|&b| (0x20..=0x7e).contains(&b) || (0x1b..=0x1e).contains(&b))
        .take(32)
        .collect();
    let mut body = vec![6, target, 0];
    body.extend(ascii);
    Ok(vec![
        sysex(&[4, target, 1])?,
        sysex(&body)?,
        sysex(&[4, target, 0x7f])?,
    ])
}
pub fn is_bitmap_ack(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xf0, 0, 0x20, 0x29, 2, 0x14, 9, 0x7f])
}
pub fn is_novation_inquiry(bytes: &[u8]) -> bool {
    bytes.len() >= 10
        && bytes[0] == 0xf0
        && bytes[1] == 0x7e
        && bytes[3..5] == [6, 2]
        && bytes[5..8] == [0, 0x20, 0x29]
        && bytes.last() == Some(&0xf7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeviceLayout;
    #[test]
    fn rgb_bytes_and_seven_bit_boundary() {
        let l = &DeviceLayout::default().leds[0];
        assert_eq!(
            rgb(l, [127, 64, 0], false).unwrap(),
            vec![240, 0, 32, 41, 2, 20, 1, 67, 96, 127, 64, 0, 247]
        );
        assert!(rgb(l, [128, 0, 0], false).is_err());
    }
    #[test]
    fn bitmap_is_row_aligned_and_padded() {
        let mut bits = vec![0; 8192];
        bits[0] = 1;
        bits[6] = 1;
        bits[127] = 1;
        bits[128] = 1;
        let b = bitmap(&bits, 32).unwrap();
        assert_eq!(b.len(), 1225);
        assert_eq!(b[8], 65);
        assert_eq!(b[26], 32);
        assert_eq!(b[27], 64);
        assert!(b[8..1224].iter().all(|v| *v <= 127));
        assert_eq!(b[1224], 247);
    }
    #[test]
    fn text_does_not_emit_utf8() {
        for msg in display_text("オーロラ Aurora", 32).unwrap() {
            assert!(msg[6..msg.len() - 1].iter().all(|&b| b < 128));
        }
    }
}
