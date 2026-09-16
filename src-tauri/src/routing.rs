use crate::model::{DeviceLayout, Settings};
#[derive(Debug, PartialEq)]
pub enum Destination {
    Pads,
    Controls,
}
pub fn route(bytes: &[u8], layout: &DeviceLayout, s: &Settings) -> Option<(Destination, Vec<u8>)> {
    if !s.forwarding || bytes.len() != 3 || bytes[1] > 127 || bytes[2] > 127 {
        return None;
    }
    let status = bytes[0] & 0xf0;
    let channel = bytes[0] & 15;
    if [0x80, 0x90, 0xa0].contains(&status) && [0, 9].contains(&channel) {
        let pads: Vec<_> = layout.leds.iter().filter(|l| l.group == "pads").collect();
        let i = pads.iter().position(|l| {
            if channel == 9 {
                l.address.drum_note == Some(bytes[1])
            } else {
                l.address.daw_note == Some(bytes[1])
            }
        })?;
        if status == 0xa0 && !s.aftertouch {
            return None;
        }
        Some((
            Destination::Pads,
            vec![status | s.pad_channel, *s.pad_notes.get(i)?, bytes[2]],
        ))
    } else if status == 0xb0
        && channel != 6
        && channel != 7
        && channel != 14
        && !s.shortcut_ccs.contains(&bytes[1])
    {
        Some((
            Destination::Controls,
            vec![
                0xb0 | s.control_channel.unwrap_or(channel),
                *s.cc_map.get(&bytes[1]).unwrap_or(&bytes[1]),
                bytes[2],
            ],
        ))
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_pad_quadrants_and_releases() {
        let l = DeviceLayout::default();
        let s = Settings::default();
        for (input, note) in [(0x70, 36), (0x60, 40), (0x74, 44), (0x64, 48)] {
            assert_eq!(
                route(&[0x90, input, 100], &l, &s),
                Some((Destination::Pads, vec![0x99, note, 100]))
            );
            assert_eq!(
                route(&[0x80, input, 0], &l, &s),
                Some((Destination::Pads, vec![0x89, note, 0]))
            );
        }
    }
    #[test]
    fn filters_control_plane_and_shortcuts() {
        let l = DeviceLayout::default();
        let mut s = Settings::default();
        s.shortcut_ccs.push(115);
        for b in [[0xb6, 29, 2], [0xbe, 1, 127], [0xbf, 115, 127]] {
            assert!(route(&b, &l, &s).is_none());
        }
        assert_eq!(
            route(&[0xbf, 85, 72], &l, &s),
            Some((Destination::Controls, vec![0xbf, 85, 72]))
        );
    }
}
