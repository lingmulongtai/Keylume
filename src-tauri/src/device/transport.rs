use std::collections::{HashMap, VecDeque};
pub trait LedTransport: Send {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}
#[derive(Default)]
pub struct MockTransport {
    pub messages: VecDeque<Vec<u8>>,
    pub leds: HashMap<(u8, u8), [u8; 3]>,
    pub daw: bool,
    pub connected: bool,
}
impl MockTransport {
    pub fn connected() -> Self {
        Self {
            connected: true,
            ..Self::default()
        }
    }
}
impl LedTransport for MockTransport {
    fn send_raw(&mut self, b: &[u8]) -> Result<(), String> {
        if !self.connected {
            return Err("Mock device disconnected".into());
        }
        if self.messages.len() == 512 {
            self.messages.pop_front();
        }
        self.messages.push_back(b.to_vec());
        if b == super::constants::DAW_ON {
            self.daw = true;
        } else if b == super::constants::DAW_OFF {
            self.daw = false;
        } else if b.len() == 13 && b.starts_with(&super::constants::HEADER) && b[6] == 1 {
            self.leds.insert((b[7], b[8]), [b[9], b[10], b[11]]);
        } else if b.len() == 3 {
            let group = if b[0] & 0xf0 == 0x90 {
                0x43
            } else if b[0] & 0xf0 == 0xb0 && b[0] & 15 < 3 {
                0x53
            } else {
                b[0]
            };
            self.leds.insert((group, b[1]), [b[2]; 3]);
        }
        Ok(())
    }
}
// Single latest-frame slot: producer never accumulates stale animation frames.
#[derive(Default)]
pub struct LatestFrame {
    pub pending: Option<Vec<[u8; 3]>>,
    pub dropped: u64,
}
impl LatestFrame {
    pub fn push(&mut self, f: Vec<[u8; 3]>) {
        if self.pending.replace(f).is_some() {
            self.dropped += 1;
        }
    }
    pub fn take(&mut self) -> Option<Vec<[u8; 3]>> {
        self.pending.take()
    }
}
pub fn differences<'a>(
    old: &'a [[u8; 3]],
    new: &'a [[u8; 3]],
    full: bool,
) -> impl Iterator<Item = (usize, [u8; 3])> + 'a {
    new.iter()
        .copied()
        .enumerate()
        .filter(move |(i, c)| full || old.get(*i) != Some(c))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_consumer_keeps_only_newest_frame() {
        let mut q = LatestFrame::default();
        for i in 0..100 {
            q.push(vec![[i; 3]]);
        }
        assert_eq!(q.take(), Some(vec![[99; 3]]));
        assert_eq!(q.dropped, 99);
        assert!(q.take().is_none());
    }
    #[test]
    fn diff_uses_quantized_values() {
        assert_eq!(
            differences(&[[1; 3], [2; 3]], &[[1; 3], [3; 3]], false).collect::<Vec<_>>(),
            vec![(1, [3; 3])]
        );
    }
    #[test]
    fn mock_records_and_interprets_wire_bytes() {
        let mut t = MockTransport::connected();
        t.send_raw(&super::super::constants::DAW_ON).unwrap();
        t.send_raw(&[240, 0, 32, 41, 2, 20, 1, 67, 96, 1, 2, 3, 247])
            .unwrap();
        assert!(t.daw);
        assert_eq!(t.leds[&(67, 96)], [1, 2, 3]);
    }
}
