use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
pub trait LedTransport: Send {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}

enum OutputCommand {
    Frame,
    Immediate(
        Vec<u8>,
        std::time::Instant,
        crossbeam_channel::Sender<Result<(), String>>,
    ),
    Stop,
}
type Pending = std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<(u8, u8), Vec<u8>>>>;
/// Hardware output has its own thread. Pending LED updates replace older colors
/// by address, so a slow driver cannot accumulate stale frames or block input.
pub struct QueuedTransport {
    pending: Pending,
    commands: crossbeam_channel::Sender<OutputCommand>,
    failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    stopped: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl QueuedTransport {
    pub fn new(mut sink: impl LedTransport + 'static) -> Self {
        let pending: Pending = Default::default();
        let updates = pending.clone();
        let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let error = failure.clone();
        let stopped = std::sync::Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let (commands, receiver) = crossbeam_channel::bounded(2);
        let thread = std::thread::spawn(move || {
            'worker: while let Ok(command) = receiver.recv() {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let batch = std::mem::take(&mut *updates.lock().unwrap());
                for bytes in batch.into_values() {
                    if stop.load(Ordering::Acquire) {
                        break 'worker;
                    }
                    if let Err(e) = sink.send_raw(&bytes) {
                        *error.lock().unwrap() = Some(e);
                    }
                }
                match command {
                    OutputCommand::Frame => {
                        let _ = sink.flush();
                    }
                    OutputCommand::Immediate(bytes, deadline, reply) => {
                        let result = if stop.load(Ordering::Acquire)
                            || std::time::Instant::now() >= deadline
                        {
                            Err("MIDI command expired before transmission".into())
                        } else {
                            sink.send_raw(&bytes)
                        };
                        let _ = reply.send(result);
                    }
                    OutputCommand::Stop => break,
                }
            }
        });
        Self {
            pending,
            commands,
            failure,
            stopped,
            thread: Some(thread),
        }
    }
    fn led_key(bytes: &[u8]) -> Option<(u8, u8)> {
        if bytes.len() == 13
            && bytes.starts_with(&super::constants::HEADER)
            && bytes[6] == 1
            && [0x43, 0x53].contains(&bytes[7])
        {
            return Some((bytes[7], bytes[8]));
        }
        if bytes.len() == 3 {
            return match bytes[0] {
                0x90..=0x92 | 0x99..=0x9b => Some((0x43, bytes[1])),
                0xb0..=0xb2 => Some((0x53, bytes[1])),
                0xb3 | 0x93 => Some((0xb3, bytes[1])),
                _ => None,
            };
        }
        None
    }
}
impl LedTransport for QueuedTransport {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), String> {
        if let Some(key) = Self::led_key(bytes) {
            if let Some(e) = self.failure.lock().unwrap().clone() {
                return Err(e);
            }
            self.pending.lock().unwrap().insert(key, bytes.to_vec());
            return Ok(());
        }
        let (tx, rx) = crossbeam_channel::bounded(1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        self.commands
            .send_deadline(
                OutputCommand::Immediate(bytes.to_vec(), deadline, tx),
                deadline,
            )
            .map_err(|e| e.to_string())?;
        rx.recv_deadline(deadline).map_err(|e| e.to_string())?
    }
    fn flush(&mut self) -> Result<(), String> {
        if let Some(e) = self.failure.lock().unwrap().clone() {
            return Err(e);
        }
        // At most two frame notifications plus a bounded latest-value map.
        match self.commands.try_send(OutputCommand::Frame) {
            Ok(()) | Err(crossbeam_channel::TrySendError::Full(_)) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
impl Drop for QueuedTransport {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.commands.try_send(OutputCommand::Stop);
        if let Some(thread) = self.thread.take() {
            // A native MIDI driver can block indefinitely. Detach a busy worker;
            // dropping the sender closes its queue once the driver returns.
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
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
    fn expired_or_stopped_commands_are_not_sent_after_driver_recovery() {
        struct Sink {
            started: crossbeam_channel::Sender<()>,
            resume: Option<crossbeam_channel::Receiver<()>>,
            dropped: crossbeam_channel::Sender<()>,
            log: std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
        }
        impl LedTransport for Sink {
            fn send_raw(&mut self, bytes: &[u8]) -> Result<(), String> {
                if let Some(resume) = self.resume.take() {
                    self.started.send(()).unwrap();
                    let _ = resume.recv();
                }
                self.log.lock().unwrap().push(bytes.to_vec());
                Ok(())
            }
        }
        impl Drop for Sink {
            fn drop(&mut self) {
                let _ = self.dropped.send(());
            }
        }
        for stop in [false, true] {
            let (started_tx, started_rx) = crossbeam_channel::bounded(1);
            let (resume_tx, resume_rx) = crossbeam_channel::bounded(1);
            let (dropped_tx, dropped_rx) = crossbeam_channel::bounded(1);
            let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let mut transport = QueuedTransport::new(Sink {
                started: started_tx,
                resume: Some(resume_rx),
                dropped: dropped_tx,
                log: log.clone(),
            });
            transport.send_raw(&[0x90, 96, 127]).unwrap();
            transport.flush().unwrap();
            started_rx
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap();
            assert!(transport
                .send_raw(&super::super::constants::DAW_OFF)
                .is_err());
            if stop {
                drop(transport);
                resume_tx.send(()).unwrap();
            } else {
                resume_tx.send(()).unwrap();
                transport
                    .send_raw(&super::super::constants::DAW_ON)
                    .unwrap();
                drop(transport);
            }
            dropped_rx
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap();
            let sent = log.lock().unwrap();
            assert!(!sent.iter().any(|b| b == &super::super::constants::DAW_OFF));
            assert_eq!(sent.len(), if stop { 1 } else { 2 });
        }
    }
    #[test]
    fn stalled_driver_bounds_immediate_commands_and_drop() {
        struct StalledSink {
            started: crossbeam_channel::Sender<()>,
            resume: crossbeam_channel::Receiver<()>,
            first: bool,
        }
        impl LedTransport for StalledSink {
            fn send_raw(&mut self, _: &[u8]) -> Result<(), String> {
                if self.first {
                    self.first = false;
                    self.started.send(()).unwrap();
                    let _ = self.resume.recv();
                }
                Ok(())
            }
        }
        let (started_tx, started_rx) = crossbeam_channel::bounded(1);
        let (resume_tx, resume_rx) = crossbeam_channel::bounded(1);
        let mut transport = QueuedTransport::new(StalledSink {
            started: started_tx,
            resume: resume_rx,
            first: true,
        });
        transport.send_raw(&[0x90, 96, 127]).unwrap();
        transport.flush().unwrap();
        started_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        transport.flush().unwrap();
        transport.flush().unwrap();
        let start = std::time::Instant::now();
        assert!(transport
            .send_raw(&super::super::constants::DAW_OFF)
            .is_err());
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
        let start = std::time::Instant::now();
        drop(transport);
        assert!(start.elapsed() < std::time::Duration::from_millis(500));
        resume_tx.send(()).unwrap();
    }
    #[test]
    fn hardware_queue_keeps_latest_colors_and_orders_release() {
        struct Sink(std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>);
        impl LedTransport for Sink {
            fn send_raw(&mut self, b: &[u8]) -> Result<(), String> {
                std::thread::sleep(std::time::Duration::from_millis(1));
                self.0.lock().unwrap().push(b.to_vec());
                Ok(())
            }
        }
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut transport = QueuedTransport::new(Sink(log.clone()));
        for color in 0..100 {
            transport
                .send_raw(&[240, 0, 32, 41, 2, 20, 1, 67, 96, color, 0, 0, 247])
                .unwrap();
        }
        assert_eq!(transport.pending.lock().unwrap().len(), 1);
        transport.flush().unwrap();
        transport
            .send_raw(&super::super::constants::DAW_OFF)
            .unwrap();
        let messages = log.lock().unwrap();
        assert_eq!(messages[0][9], 99);
        assert_eq!(messages.last().unwrap(), &super::super::constants::DAW_OFF);
    }
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
