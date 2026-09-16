use super::transport::{LedTransport, QueuedTransport};
use crossbeam_channel::Sender;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
pub static INPUT_OVERFLOW: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Debug)]
pub struct MidiPacket {
    pub source: String,
    pub bytes: Vec<u8>,
    pub received_at: std::time::Instant,
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Ports {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}
pub fn ports() -> Result<Ports, String> {
    let i = MidiInput::new("Keylume discovery").map_err(|e| e.to_string())?;
    let o = MidiOutput::new("Keylume discovery").map_err(|e| e.to_string())?;
    Ok(Ports {
        inputs: i
            .ports()
            .iter()
            .filter_map(|p| i.port_name(p).ok())
            .collect(),
        outputs: o
            .ports()
            .iter()
            .filter_map(|p| o.port_name(p).ok())
            .collect(),
    })
}
fn compatible(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("launchkey") && n.contains("mk4") && n.contains("61")
}
fn daw_hint(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("daw")
        || n.contains("midi 2")
        || n.contains("midi2")
        || n.ends_with(" da")
        || n.contains("midiin2")
        || n.contains("midiout2")
}
fn pick(names: &[String], daw: bool) -> Option<usize> {
    let matches: Vec<_> = names
        .iter()
        .enumerate()
        .filter(|(_, n)| compatible(n))
        .collect();
    if daw {
        matches
            .iter()
            .find(|(_, n)| daw_hint(n))
            .or_else(|| matches.get(1))
            .map(|(i, _)| *i)
    } else {
        matches.iter().find(|(_, n)| !daw_hint(n)).map(|(i, _)| *i)
    }
}
pub struct HardwareTransport {
    output: QueuedTransport,
    pub name: String,
    _inputs: Vec<MidiInputConnection<()>>,
}
impl HardwareTransport {
    pub fn connect(tx: Sender<MidiPacket>, keyboard: bool) -> Result<Self, String> {
        let mut input = MidiInput::new("Keylume DAW input").map_err(|e| e.to_string())?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new("Keylume lights").map_err(|e| e.to_string())?;
        let ips = input.ports();
        let ops = output.ports();
        let ins: Vec<_> = ips
            .iter()
            .map(|p| input.port_name(p).unwrap_or_default())
            .collect();
        let outs: Vec<_> = ops
            .iter()
            .map(|p| output.port_name(p).unwrap_or_default())
            .collect();
        let error = || {
            if ins
                .iter()
                .chain(&outs)
                .any(|n| n.to_lowercase().contains("launchkey"))
            {
                "未対応モデル、または 61 鍵 DAW ポートを特定できません".to_string()
            } else {
                "Launchkey MK4 61 が接続されていません".to_string()
            }
        };
        let di = pick(&ins, true).ok_or_else(error)?;
        let do_ = pick(&outs, true).ok_or_else(error)?;
        let key = pick(&ins, false).filter(|i| *i != di);
        let tx2 = tx.clone();
        let connection = input
            .connect(
                &ips[di],
                "Keylume DAW",
                move |_, b, _| {
                    if tx2
                        .try_send(MidiPacket {
                            source: "daw".into(),
                            bytes: b.to_vec(),
                            received_at: std::time::Instant::now(),
                        })
                        .is_err()
                    {
                        INPUT_OVERFLOW.store(true, Ordering::SeqCst);
                    }
                },
                (),
            )
            .map_err(|e| format!("DAW ポートを開けません: {e}"))?;
        let mut inputs = vec![connection];
        if keyboard {
            if let Some(k) = key {
                let mut i = MidiInput::new("Keylume keyboard").map_err(|e| e.to_string())?;
                i.ignore(Ignore::None);
                let ps = i.ports();
                let p = ps.get(k).ok_or("MIDI ポートが切断されました")?;
                if let Ok(c) = i.connect(
                    p,
                    "Keylume keyboard",
                    move |_, b, _| {
                        if tx
                            .try_send(MidiPacket {
                                source: "keyboard".into(),
                                bytes: b.to_vec(),
                                received_at: std::time::Instant::now(),
                            })
                            .is_err()
                        {
                            INPUT_OVERFLOW.store(true, Ordering::SeqCst);
                        }
                    },
                    (),
                ) {
                    inputs.push(c);
                }
            }
        }
        let out = output
            .connect(&ops[do_], "Keylume lights")
            .map_err(|e| format!("DAW ポートを開けません: {e}"))?;
        Ok(Self {
            output: QueuedTransport::new(MidiSink(out)),
            name: outs[do_].clone(),
            _inputs: inputs,
        })
    }
}
impl LedTransport for HardwareTransport {
    fn send_raw(&mut self, b: &[u8]) -> Result<(), String> {
        self.output.send_raw(b)
    }
    fn flush(&mut self) -> Result<(), String> {
        self.output.flush()
    }
}
struct MidiSink(MidiOutputConnection);
impl LedTransport for MidiSink {
    fn send_raw(&mut self, b: &[u8]) -> Result<(), String> {
        self.0.send(b).map_err(|e| e.to_string())
    }
}
pub fn open_forward(name: &str) -> Result<MidiOutputConnection, String> {
    if name.trim().is_empty() {
        return Err("転送ポート未選択".into());
    }
    if name.to_lowercase().contains("launchkey") {
        return Err("実機ポートを転送先には指定できません".into());
    }
    let midi = MidiOutput::new("Keylume forwarding").map_err(|e| e.to_string())?;
    let matches: Vec<_> = midi
        .ports()
        .into_iter()
        .filter(|p| midi.port_name(p).is_ok_and(|n| n == name))
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "転送ポート「{name}」が見つからないか、同名が複数あります"
        ));
    }
    midi.connect(&matches[0], "Keylume forwarding")
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_second_interface_is_identified_without_relying_on_order() {
        for prefix in ["MIDIIN2", "MIDIOUT2"] {
            let names = vec![
                format!("{prefix} (Launchkey MK4 61 MIDI)"),
                "Launchkey MK4 61 MIDI".into(),
            ];
            assert_eq!(pick(&names, true), Some(0));
            assert_eq!(pick(&names, false), Some(1));
        }
        assert_eq!(pick(&["Launchkey MK4 49 DAW".into()], true), None);
    }
}
