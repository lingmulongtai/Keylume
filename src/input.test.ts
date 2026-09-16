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
