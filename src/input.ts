import type { Color, Layout } from './types';
export interface InputState {
  encoders: (number | null)[];
  relative: boolean[];
  faders: (number | null)[];
  pitch: number | null;
  modulation: number | null;
  sustain: number | null;
  pressure: number | null;
  polyPressure: Record<string, number>;
  features: Record<string, number>;
  held: string[];
  encoderMode: number | null;
  faderMode: number | null;
  lastNote: [number, number] | null;
  lastMessage: string;
}
export const emptyInput = (): InputState => ({
  encoders: Array(8).fill(null),
  relative: Array(8).fill(false),
  faders: Array(9).fill(null),
  pitch: null,
  modulation: null,
  sustain: null,
  pressure: null,
  polyPressure: {},
  features: {},
  held: [],
  encoderMode: null,
  faderMode: null,
  lastNote: null,
  lastMessage: '',
});
// The two transport lamps are monochrome hardware; only their intensity changes.
export function hardwareColor(id: string, color: Color): Color {
  const intensity = Math.max(...color);
  return id === 'btn.play' ? [0, intensity, 0] : id === 'btn.record' ? [intensity, 0, 0] : color;
}
export class PreviewInput {
  state = emptyInput();
  private presses = new Map<string, string>();
  private pressureSource = '';
  reset() {
    this.state = emptyInput();
    this.presses.clear();
    this.pressureSource = '';
  }
  clearPort(source: string) {
    for (const key of this.presses.keys())
      if (key.startsWith(source + ':')) this.presses.delete(key);
    const s = this.state;
    s.held = [...new Set(this.presses.values())].sort();
    if (source === 'daw') {
      s.encoders.fill(null);
      s.relative.fill(false);
      s.faders.fill(null);
      s.encoderMode = null;
      s.faderMode = null;
      s.features = {};
      for (const id of Object.keys(s.polyPressure))
        if (!id.startsWith('key.')) delete s.polyPressure[id];
    }
    if (source === 'keyboard') {
      s.pitch = null;
      s.modulation = null;
      s.sustain = null;
      for (const id of Object.keys(s.polyPressure))
        if (id.startsWith('key.')) delete s.polyPressure[id];
    }
    if (this.pressureSource === source) {
      s.pressure = null;
      this.pressureSource = '';
    }
  }
  receive(source: string, b: number[], layout: Layout) {
    const s = this.state,
      kind = b[0] & 0xf0,
      ch = b[0] & 15,
      v = b[2] ?? 0;
    if (
      b[0] < 128 ||
      b[0] >= 240 ||
      b.length !== ([0xc0, 0xd0].includes(kind) ? 2 : 3) ||
      b.slice(1).some((v) => v < 0 || v > 127)
    )
      return;
    const keyboard = source === 'keyboard' || source === 'screen';
    const key = `${source}:${ch}:${b[1]}`;
    s.lastMessage = `${source} · ${b.map((v) => v.toString(16).toUpperCase().padStart(2, '0')).join(' ')}`;
    if ([0x80, 0x90, 0xa0].includes(kind)) {
      const id = keyboard
        ? `key.${b[1]}`
        : layout.leds.find(
            (l) =>
              l.group === 'pads' && (ch === 9 ? l.address.drumNote : l.address.dawNote) === b[1],
          )?.id;
      if (id) {
        if (kind === 0xa0) s.polyPressure[id] = v;
        else if (kind === 0x90 && v) {
          this.presses.set(key, id);
          if (keyboard) s.lastNote = [b[1], v];
        } else {
          this.presses.delete(key);
          delete s.polyPressure[id];
        }
      }
    } else if (source === 'daw' && kind === 0xd0) {
      s.pressure = b[1];
      this.pressureSource = source;
    } else if (source === 'daw' && b[0] === 0xbe) {
      const id =
        b[1] >= 5 && b[1] <= 13
          ? `fader-${b[1] - 4}`
          : b[1] >= 21 && b[1] <= 28
            ? `encoder-${b[1] - 20}`
            : b[1] >= 85 && b[1] <= 92
              ? `encoder-${b[1] - 84}`
              : null;
      if (id) {
        if (v) this.presses.set(key, id);
        else this.presses.delete(key);
      }
    } else if (keyboard) {
      if (kind === 0xe0) s.pitch = b[1] + (v << 7);
      if (kind === 0xd0) {
        s.pressure = b[1];
        this.pressureSource = source;
      }
      if (kind === 0xb0) {
        if (b[1] === 1) s.modulation = v;
        if (b[1] === 64) s.sustain = v;
        if ([120, 123].includes(b[1]))
          for (const key of this.presses.keys()) {
            if (key.startsWith(`${source}:${ch}:`)) this.presses.delete(key);
          }
        if (b[1] === 121) {
          s.sustain = 0;
          s.pitch = 8192;
          s.modulation = 0;
        }
      }
    } else if (source === 'daw' && b[0] === 0xbf) {
      if (b[1] >= 5 && b[1] <= 13) s.faders[b[1] - 5] = v;
      else if (b[1] >= 21 && b[1] <= 28) {
        s.encoders[b[1] - 21] = v;
        s.relative[b[1] - 21] = false;
      } else if (b[1] >= 85 && b[1] <= 92) {
        const i = b[1] - 85;
        s.encoders[i] = ((((s.relative[i] ? (s.encoders[i] ?? 0) : 0) + v - 64) % 128) + 128) % 128;
        s.relative[i] = true;
      } else {
        const id = b[1] === 63 ? 'btn.shift' : layout.leds.find((l) => l.address.cc === b[1])?.id;
        if (id) {
          if (v) this.presses.set(key, id);
          else this.presses.delete(key);
        }
      }
    } else if (source === 'daw' && b[0] === 0xb6) {
      s.features[b[1]] = v;
      if (b[1] === 63) {
        if (v) this.presses.set(key, 'btn.shift');
        else this.presses.delete(key);
      }
      if (b[1] === 30) {
        if (s.encoderMode !== v) {
          s.encoders.fill(null);
          s.relative.fill(false);
        }
        s.encoderMode = v;
      }
      if (b[1] === 31) {
        if (s.faderMode !== v) s.faders.fill(null);
        s.faderMode = v;
      }
    }
    s.held = [...new Set(this.presses.values())].sort();
  }
}
