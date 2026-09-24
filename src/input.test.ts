import { expect, test } from 'vitest';
import { PreviewInput, hardwareColor } from './input';
import layout from '../resources/layout.json';
import type { Layout } from './types';
test('decodes DAW controls without mistaking keyboard CCs or mode reports for faders', () => {
  const input = new PreviewInput(),
    l = layout as Layout;
  input.receive('keyboard', [0xb0, 5, 127], l);
  input.receive('daw', [0xb6, 5, 127], l);
  expect(input.state.faders[0]).toBeNull();
  input.receive('daw', [0xbf, 5, 127], l);
  input.receive('daw', [0xbf, 21, 32], l);
  expect(input.state.faders[0]).toBe(127);
  expect(input.state.encoders[0]).toBe(32);
  input.receive('daw', [0xbf, 85, 65], l);
  input.receive('daw', [0xbf, 85, 63], l);
  expect(input.state.encoders[0]).toBe(0);
  expect(input.state.relative[0]).toBe(true);
  input.receive('keyboard', [0xe2, 0, 64], l);
  input.receive('keyboard', [0xb2, 64, 127], l);
  expect(input.state.pitch).toBe(8192);
  expect(input.state.sustain).toBe(127);
  input.reset();
  expect(input.state.sustain).toBeNull();
  expect(input.state.faders.every((v) => v == null)).toBe(true);
});
test('keeps note sources independent and transport lamp hues fixed', () => {
  const input = new PreviewInput(),
    l = layout as Layout;
  input.receive('keyboard', [0x90, 60, 90], l);
  input.receive('screen', [0x90, 60, 100], l);
  input.receive('keyboard', [0x90, 60, 0], l);
  expect(input.state.held).toEqual(['key.60']);
  input.receive('screen', [0x80, 60, 0], l);
  expect(input.state.held).toEqual([]);
  expect(hardwareColor('btn.play', [100, 20, 50])).toEqual([0, 100, 0]);
  expect(hardwareColor('btn.record', [0, 80, 127])).toEqual([127, 0, 0]);
  expect(hardwareColor('btn.play', [0, 0, 0])).toEqual([0, 0, 0]);
});

test('touch and feature replies do not move controls or collide with DAW buttons', () => {
  const input = new PreviewInput(),
    l = layout as Layout;
  input.receive('daw', [0xbe, 5, 127], l);
  expect(input.state.held).toContain('fader-1');
  expect(input.state.faders[0]).toBeNull();
  input.receive('daw', [0xbe, 5, 0], l);
  expect(input.state.held).not.toContain('fader-1');
  input.receive('daw', [0xb6, 74, 1], l);
  expect(input.state.features['74']).toBe(1);
  expect(input.state.held).not.toContain('btn.capture');
  input.receive('daw', [0xb6, 63, 127], l);
  expect(input.state.held).toContain('btn.shift');
  input.receive('daw', [0xdf, 85], l);
  expect(input.state.pressure).toBe(85);
});

test('releasing the DAW port preserves keyboard sustain and screen notes', () => {
  const input = new PreviewInput(),
    l = layout as Layout;
  input.receive('keyboard', [0x90, 60, 100], l);
  input.receive('keyboard', [0xb0, 64, 127], l);
  input.receive('keyboard', [0xd0, 80], l);
  input.receive('screen', [0x90, 64, 100], l);
  input.receive('daw', [0xbf, 115, 127], l);
  input.receive('daw', [0xbe, 5, 127], l);
  input.receive('daw', [0xbf, 5, 100], l);
  input.clearPort('daw');
  expect(input.state.held).toEqual(['key.60', 'key.64']);
  expect(input.state.sustain).toBe(127);
  expect(input.state.pressure).toBe(80);
  expect(input.state.faders[0]).toBeNull();
  input.clearPort('keyboard');
  expect(input.state.held).toEqual(['key.64']);
  expect(input.state.sustain).toBeNull();
  expect(input.state.pressure).toBeNull();
});

test('shows real standalone button captures and gives Start/Stop a repeatable pulse', () => {
  const input = new PreviewInput(),
    l = layout as Layout;
  for (const cc of [103, 102, 77, 117, 76, 74, 75]) {
    input.receive('keyboard', [0xbf, cc, 127], l);
    expect(input.state.held).toHaveLength(1);
    input.receive('keyboard', [0xbf, cc, 0], l);
    expect(input.state.held).toEqual([]);
  }
  input.receive('keyboard', [0xfa], l);
  expect(input.state.pulse).toEqual(['btn.play', 1]);
  input.receive('keyboard', [0xfa], l);
  expect(input.state.pulse).toEqual(['btn.play', 2]);
  input.receive('keyboard', [0xfc], l);
  expect(input.state.pulse).toEqual(['btn.stop', 3]);
  expect(input.state.held).toEqual([]);
});
