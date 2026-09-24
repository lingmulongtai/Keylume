use super::{desktop_actions::DesktopActions, runtime::Core};
use crate::{
    controller::{Binding, ControlInput, Mode, EFFECT_NAMES},
    groove::LoopCommand,
    instrument_fx::InstrumentFx,
};

/// Returns whether saved preferences changed. Called outside the runtime's control lock.
pub fn perform(
    core: &Core,
    desktop: &DesktopActions,
    binding: &Binding,
    input: &ControlInput,
) -> Result<bool, String> {
    let action = binding.action.as_str();
    if action == "shortcut" || action == "open" {
        if !core.control.lock().unwrap().settings.mock {
            desktop.run(binding.clone())?;
        }
        return Ok(false);
    }
    if action == "scroll" {
        let settings = core.control.lock().unwrap();
        desktop.scroll(crate::controller::scroll_rate(
            input.value,
            settings.settings.controller.scroll_speed,
        ));
        return Ok(false);
    }
    let command = match action {
        "undo" => Some(LoopCommand::Undo),
        "capture" => Some(LoopCommand::Capture),
        "quantise" => Some(LoopCommand::Quantise),
        "loopPlay" => Some(LoopCommand::Play),
        "loopStop" => Some(LoopCommand::Stop),
        "loopOverdub" => Some(LoopCommand::Overdub),
        "loopClear" => Some(LoopCommand::Clear),
        "loopRecord" => Some(LoopCommand::RecordToggle),
        "metronome" => Some(LoopCommand::Metronome),
        "tempo" => {
            let status = core.piano.bus.loop_status();
            let bpm = input.delta.map_or(40. + input.value as f64 * 200., |d| {
                status.bpm + d as f64 * 200.
            });
            Some(LoopCommand::Tempo(bpm.round().clamp(40., 240.)))
        }
        _ => None,
    };
    if let Some(c) = command {
        let status = core.piano.bus.loop_status();
        core.piano.bus.loop_command(c)?;
        match c {
            LoopCommand::Tempo(bpm) => core.show_feedback("Tempo", format!("{bpm:.0} BPM")),
            LoopCommand::Metronome => {
                core.show_feedback("Metronome", if status.metronome { "Off" } else { "On" })
            }
            LoopCommand::Undo => core.show_feedback(
                "Undo",
                if status.can_undo {
                    "Restored"
                } else {
                    "Nothing to undo"
                },
            ),
            _ => core.show_feedback("Looper", ""),
        }
        return Ok(false);
    }
    if action == "panic" {
        core.piano.bus.panic();
        core.performance.pause();
        desktop.stop_scroll();
        return Ok(false);
    }
    let mut control = core.control.lock().unwrap();
    let next_value = |old: f32| input.delta.map_or(input.value, |d| old + d).clamp(0., 1.);
    match action {
        "mode" => {
            let mode = &mut control.settings.controller.mode;
            *mode = if *mode == Mode::Performance {
                Mode::Desktop
            } else {
                Mode::Performance
            };
            core.piano.bus.set_performing(*mode == Mode::Performance);
            core.performance.pause();
            desktop.allowed(false);
        }
        "piano" => control.settings.piano.enabled = !control.settings.piano.enabled,
        "effect" => {
            let index = EFFECT_NAMES
                .iter()
                .position(|e| *e == binding.value)
                .ok_or("エフェクトが不正です")?;
            let mut values = control.settings.piano.effects.values();
            values[index] = next_value(values[index]);
            control.settings.piano.effects = InstrumentFx::from_values(values);
        }
        "volume" => control.settings.piano.volume = next_value(control.settings.piano.volume),
        "brightness" => {
            control.settings.master_brightness = next_value(control.settings.master_brightness)
        }
        "kit" => {
            let d = binding.value.parse::<i8>().unwrap_or(1);
            let kit = &mut control.settings.piano.drum_kit.kit;
            *kit = (*kit as i8 + d).rem_euclid(crate::drums::KIT_COUNT as i8) as u8;
        }
        "sound" | "favorite" => {
            let available = core.library.entries();
            let available: Vec<_> = available
                .iter()
                .filter(|e| e.installed)
                .map(|e| &e.entry.id)
                .collect();
            let piano = &mut control.settings.piano;
            let favorites: Vec<_> = piano
                .favorites
                .iter()
                .filter(|id| available.contains(id))
                .cloned()
                .collect();
            if action == "favorite" {
                let id = piano
                    .favorites
                    .get(binding.value.parse::<usize>().unwrap_or(0))
                    .ok_or("この位置にお気に入りがありません")?;
                if !available.contains(&id) {
                    return Err("音源ライブラリでこの音色を一度ダウンロードしてください".into());
                }
                piano.sound = id.clone();
            } else {
                let choices: Vec<_> = if favorites.is_empty() {
                    available.into_iter().cloned().collect()
                } else {
                    favorites
                };
                if !choices.is_empty() {
                    let i = choices
                        .iter()
                        .position(|id| *id == piano.sound)
                        .unwrap_or(0);
                    let direction = binding.value.parse::<isize>().unwrap_or(1);
                    piano.sound = choices
                        [(i as isize + direction).rem_euclid(choices.len() as isize) as usize]
                        .clone();
                }
            }
        }
        "lighting" => {
            if !control.presets.is_empty() {
                let i = control
                    .presets
                    .iter()
                    .position(|p| p.id == control.settings.active_preset)
                    .unwrap_or(0);
                let direction = binding.value.parse::<isize>().unwrap_or(1);
                let next = control.presets
                    [(i as isize + direction).rem_euclid(control.presets.len() as isize) as usize]
                    .clone();
                control.settings.active_preset = next.id.clone();
                control.draft = next;
                control.revision += 1;
            }
        }
        _ => return Ok(false),
    }
    let p = &control.settings.piano;
    let percent = |v: f32| format!("{:.0}%", v * 100.);
    match action {
        "effect" => {
            let i = EFFECT_NAMES
                .iter()
                .position(|name| *name == binding.value)
                .unwrap();
            core.show_feedback(
                [
                    "Reverb",
                    "Delay",
                    "Filter",
                    "Resonance",
                    "Chorus",
                    "Drive",
                    "Stereo Width",
                    "Tremolo",
                ][i],
                percent(p.effects.values()[i]),
            );
        }
        "volume" => core.show_feedback("Piano Volume", percent(p.volume)),
        "brightness" => core.show_feedback(
            "Lighting Level",
            percent(control.settings.master_brightness),
        ),
        "kit" => core.show_feedback(
            "Drum Kit",
            [
                "Studio",
                "Sub 808",
                "Punch 909",
                "Lo-fi",
                "Glass",
                "Percussion",
            ][p.drum_kit.kit as usize],
        ),
        "sound" | "favorite" => {
            if let Some(entry) = core
                .library
                .entries()
                .iter()
                .find(|e| e.entry.id == p.sound)
            {
                let name = &entry.entry.name;
                core.show_feedback(
                    "Sound",
                    if name.is_ascii() {
                        name.clone()
                    } else {
                        format!("User B{} P{}", entry.entry.bank, entry.entry.program)
                    },
                );
            }
        }
        "lighting" => core.show_feedback(
            "Lighting",
            if control.draft.name.is_ascii() {
                &control.draft.name
            } else {
                &control.draft.id
            },
        ),
        "mode" => core.show_feedback(
            "Mode",
            if control.settings.controller.mode == Mode::Performance {
                "Performance"
            } else {
                "Desktop"
            },
        ),
        "piano" => core.show_feedback("Instrument", if p.enabled { "On" } else { "Off" }),
        _ => {}
    }
    core.piano.configure(&control.settings.piano);
    Ok(true)
}
