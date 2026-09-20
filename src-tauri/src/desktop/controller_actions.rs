use super::{desktop_actions::DesktopActions, runtime::Core};
use crate::{
    controller::{Binding, ControlInput, Mode, EFFECT_NAMES},
    groove::{LoopCommand, LoopConfig},
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
        "loopPlay" => Some(LoopCommand::Play),
        "loopStop" => Some(LoopCommand::Stop),
        "loopOverdub" => Some(LoopCommand::Overdub),
        "loopClear" => Some(LoopCommand::Clear),
        "loopRecord" => {
            let status = core.piano.bus.loop_status();
            Some(if status.mode == "playing" || status.mode == "overdub" {
                LoopCommand::Overdub
            } else if status.mode == "recording" || status.mode == "countIn" {
                LoopCommand::Stop
            } else {
                LoopCommand::Record(LoopConfig {
                    bpm: status.bpm,
                    bars: (status.beats / 4.) as u8,
                    metronome: status.metronome,
                })
            })
        }
        _ => None,
    };
    if let Some(c) = command {
        core.piano.bus.loop_command(c)?;
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
    core.piano.configure(&control.settings.piano);
    Ok(true)
}
