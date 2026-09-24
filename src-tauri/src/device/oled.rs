use super::{protocol, transport::LedTransport};
use crossbeam_channel::{Receiver, TryRecvError};

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Off,
    Text(String),
    Feedback(String, String),
    Bitmap(Vec<u8>),
}
impl Content {
    fn messages(&self) -> Result<Vec<Vec<u8>>, String> {
        // Always cancel the hardware's higher-priority temporary page first.
        let mut result = vec![protocol::sysex(&[4, 0x21, 0])?];
        match self {
            Self::Off => result.push(protocol::sysex(&[4, 0x20, 0])?),
            Self::Text(text) => result.extend(protocol::display_text(text, 0x20)?),
            Self::Feedback(title, value) => {
                result.extend(protocol::display_lines(title, value, 0x20)?)
            }
            Self::Bitmap(bits) => result.push(protocol::bitmap(bits, 0x20)?),
        }
        Ok(result)
    }
}
struct Flight {
    content: Content,
    messages: Vec<Vec<u8>>,
    completion: Receiver<Result<(), String>>,
    early_ack: bool,
}
#[derive(Default)]
pub struct Progress {
    pub sent: Vec<Vec<u8>>,
    pub warning: Option<String>,
}
pub struct DisplayQueue {
    desired: Content,
    last: Option<Content>,
    flight: Option<Flight>,
    awaiting_ack: Option<f64>,
    warned: bool,
    retry_at: f64,
}
impl Default for DisplayQueue {
    fn default() -> Self {
        Self {
            desired: Content::Off,
            last: None,
            flight: None,
            awaiting_ack: None,
            warned: false,
            retry_at: 0.,
        }
    }
}
impl DisplayQueue {
    pub fn request(&mut self, content: Content) {
        self.desired = content;
    }
    pub fn reset(&mut self) {
        let desired = self.desired.clone();
        *self = Self::default();
        self.desired = desired;
    }
    pub fn acknowledge(&mut self) {
        if self.awaiting_ack.take().is_none() {
            if let Some(f) = &mut self.flight {
                if matches!(f.content, Content::Bitmap(_)) {
                    f.early_ack = true;
                }
            }
        }
        self.warned = false;
    }
    pub fn pump(&mut self, now: f64, output: &mut dyn LedTransport, mock: bool) -> Progress {
        let mut progress = Progress::default();
        let completion = self
            .flight
            .as_ref()
            .and_then(|f| match f.completion.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("OLED送信が中断されました".into())),
            });
        if let Some(result) = completion {
            let flight = self.flight.take().unwrap();
            match result {
                Ok(()) => {
                    if matches!(flight.content, Content::Bitmap(_)) && !flight.early_ack && !mock {
                        self.awaiting_ack = Some(now);
                        self.warned = false;
                    }
                    self.last = Some(flight.content);
                    progress.sent = flight.messages;
                }
                Err(error) => {
                    progress.warning = Some(error);
                    self.retry_at = now + 0.1;
                }
            }
        }
        if self.awaiting_ack.is_some_and(|at| now - at > 2.) && !self.warned {
            self.warned = true;
            progress.warning=Some("OLED画像の応答待ちです。文字表示は継続できます。応答受信または再接続で画像を再開します".into());
        }
        if self.flight.is_none()
            && now >= self.retry_at
            && self.last.as_ref() != Some(&self.desired)
            && (!matches!(self.desired, Content::Bitmap(_))
                || self.awaiting_ack.is_none()
                || matches!(self.last, Some(Content::Feedback(_, _))))
        {
            // Expired operation feedback must clear even while a bitmap ACK is missing.
            let content =
                if matches!(self.desired, Content::Bitmap(_)) && self.awaiting_ack.is_some() {
                    Content::Text(String::new())
                } else {
                    self.desired.clone()
                };
            let messages = match content.messages() {
                Ok(messages) => messages,
                Err(error) => {
                    progress.warning = Some(error);
                    self.retry_at = now + 1.;
                    return progress;
                }
            };
            let completion = output.submit(messages.clone());
            self.flight = Some(Flight {
                content,
                messages,
                completion,
                early_ack: false,
            });
        }
        progress
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expired_feedback_clears_before_a_missing_bitmap_ack_recovers() {
        let mut q = DisplayQueue::default();
        let mut out = Deferred::default();
        q.request(image(0));
        q.pump(0., &mut out, false);
        out.replies[0].send(Ok(())).unwrap();
        q.pump(0.1, &mut out, false);
        q.request(Content::Feedback("Piano Volume".into(), "75%".into()));
        q.pump(0.2, &mut out, false);
        assert!(out.batches[1].contains(&protocol::sysex(&[6, 32, 1, b'7', b'5', b'%']).unwrap()));
        out.replies[1].send(Ok(())).unwrap();
        q.pump(0.3, &mut out, false);
        q.request(image(1));
        q.pump(3., &mut out, false);
        assert_eq!(
            out.batches[2],
            Content::Text(String::new()).messages().unwrap()
        );
        out.replies[2].send(Ok(())).unwrap();
        q.pump(3.1, &mut out, false);
        assert_eq!(out.batches.len(), 3);
        q.acknowledge();
        q.pump(3.2, &mut out, false);
        assert_eq!(out.batches[3], image(1).messages().unwrap());
    }
    #[derive(Default)]
    struct Deferred {
        batches: Vec<Vec<Vec<u8>>>,
        replies: Vec<crossbeam_channel::Sender<Result<(), String>>>,
    }
    impl LedTransport for Deferred {
        fn send_raw(&mut self, _: &[u8]) -> Result<(), String> {
            panic!("OLED must not use blocking send")
        }
        fn submit(&mut self, messages: Vec<Vec<u8>>) -> Receiver<Result<(), String>> {
            let (tx, rx) = crossbeam_channel::bounded(1);
            self.batches.push(messages);
            self.replies.push(tx);
            rx
        }
    }
    fn image(n: usize) -> Content {
        let mut bits = vec![0; 8192];
        bits[n] = 1;
        Content::Bitmap(bits)
    }
    #[test]
    fn rapid_changes_keep_one_in_flight_and_send_only_the_latest_after_ack() {
        let mut q = DisplayQueue::default();
        let mut out = Deferred::default();
        q.request(image(0));
        q.pump(0., &mut out, false);
        q.request(image(1));
        q.request(image(2));
        q.pump(0.01, &mut out, false);
        assert_eq!(out.batches.len(), 1);
        out.replies[0].send(Ok(())).unwrap();
        q.pump(0.02, &mut out, false);
        assert_eq!(out.batches.len(), 1);
        q.acknowledge();
        q.pump(0.03, &mut out, false);
        assert_eq!(out.batches.len(), 2);
        assert_eq!(out.batches[1], image(2).messages().unwrap());
    }
    #[test]
    fn text_survives_missing_bitmap_ack_and_failed_text_is_retried() {
        let mut q = DisplayQueue::default();
        let mut out = Deferred::default();
        q.request(image(0));
        q.pump(0., &mut out, false);
        out.replies[0].send(Ok(())).unwrap();
        q.pump(0.1, &mut out, false);
        q.request(Content::Text("new".into()));
        assert!(q.pump(3., &mut out, false).warning.is_some());
        assert_eq!(out.batches.len(), 2);
        out.replies[1].send(Err("driver".into())).unwrap();
        q.pump(3.01, &mut out, false);
        q.pump(3.2, &mut out, false);
        assert_eq!(out.batches.len(), 3);
        out.replies[2].send(Ok(())).unwrap();
        q.pump(3.3, &mut out, false);
        q.request(image(4));
        q.pump(3.4, &mut out, false);
        assert_eq!(out.batches.len(), 3);
        q.acknowledge();
        q.pump(3.5, &mut out, false);
        assert_eq!(out.batches[3], image(4).messages().unwrap());
    }
    #[test]
    fn an_ack_before_send_completion_is_not_lost() {
        let mut q = DisplayQueue::default();
        let mut out = Deferred::default();
        q.request(image(0));
        q.pump(0., &mut out, false);
        q.acknowledge();
        out.replies[0].send(Ok(())).unwrap();
        q.request(image(1));
        q.pump(0.1, &mut out, false);
        assert_eq!(out.batches.len(), 2);
    }
}
