import { describe, it, expect, vi } from 'vitest';
import { compose, renderPreview, ditherImage, previewInput } from './preview';
import { keyPosition, lightingPosition } from './layout';
import { validatePreset, validateLayout } from './api';
import data from '../resources/presets.json';
import layout from '../resources/layout.json';
import type { Preset, Layout } from './types';
describe('preview and interchange', () => {
  it('fades released rainbow notes and keeps opposing notes colorful', () => {
    const l = layout as Layout;
    const p = structuredClone(data.find((p) => p.id === 'note-rainbow')) as Preset;
    p.layers = [p.layers[0]];
    p.layers[0].blend = 'normal';
    p.post = { brightness: 1, saturation: 1, temperatureK: 6500, gamma: 1 };
    const x = (keyPosition(l, 60) - keyPosition(l, 36)) / (keyPosition(l, 96) - keyPosition(l, 36));
    const index = l.leds
      .map((led, i) => ({
        i,
        d: Math.abs(lightingPosition(l, i) - x),
        kind: led.kind,
      }))
      .filter((v) => v.kind === 'rgb')
      .sort((a, b) => a.d - b.d)[0].i;
    let at = 0;
    const clock = vi.spyOn(performance, 'now').mockImplementation(() => at * 1000);
    try {
      previewInput([0xb0, 123, 0], 'keyboard', l);
      previewInput([0x90, 60, 100], 'keyboard', l);
      const held = renderPreview(p, l, at, 1)[index];
      expect(Math.max(...held)).toBeGreaterThan(0);
      at = 1;
      previewInput([0x80, 60, 0], 'keyboard', l);
      expect(renderPreview(p, l, at, 1)[index]).toEqual(held);
      at = 1.275;
      expect(
        Math.abs(Math.max(...renderPreview(p, l, at, 1)[index]) - Math.max(...held) / 2),
      ).toBeLessThanOrEqual(1);
      at = 1.6;
      expect(renderPreview(p, l, at, 1)[index]).toEqual([0, 0, 0]);
      previewInput([0x90, 60, 100], 'keyboard', l);
      previewInput([0x90, 66, 100], 'keyboard', l);
      for (const color of renderPreview(p, l, at, 1))
        if (Math.max(...color) > 50 && color[0] !== color[1])
          expect(Math.min(...color)).toBeLessThan(12);
    } finally {
      previewInput([0xb0, 123, 0], 'keyboard', l);
      clock.mockRestore();
    }
  });
  it('breathes through the selected color stops using the native phase', () => {
    const p = structuredClone(data[0]) as Preset;
    p.layers = [
      {
        ...p.layers[0],
        effect: 'breathing',
        params: { colors: ['#ff0000', '#0000ff'], period: 6 },
        opacity: 1,
        zone: 'all',
      },
    ];
    p.post = { brightness: 1, saturation: 1, temperatureK: 6500, gamma: 1 };
    expect(renderPreview(p, layout as Layout, 1.5, 1)[0]).toEqual([116, 0, 11]);
    expect(renderPreview(p, layout as Layout, 7.5, 1)[0]).toEqual([74, 0, 53]);
  });
  it('rejects malformed layouts and LED kinds with missing addresses', () => {
    expect(validateLayout(layout).leds).toHaveLength(42);
    for (const invalid of [
      { ...layout, leds: [] },
      { ...layout, decor: {} },
      { ...layout, canvas: { w: 0, h: 1 } },
      { ...layout, leds: [layout.leds[0], layout.leds[0]] },
      { ...layout, leds: [{ ...layout.leds[0], kind: 'mono' }] },
      {
        ...layout,
        leds: [{ ...layout.leds[0], address: { ...layout.leds[0].address, sysexId: 128 } }],
      },
    ])
      expect(() => validateLayout(invalid)).toThrow();
    expect(validateLayout({ ...layout, leds: [layout.leds[0]] }).leds).toHaveLength(1);
  });
  it('composes layers with transparent opacity', () => {
    for (const mode of ['normal', 'add', 'multiply', 'screen', 'max'])
      expect(compose([0.2, 0.2, 0.2], [0.7, 0.7, 0.7], 0, mode)).toEqual([0.2, 0.2, 0.2]);
    expect(compose([0.2, 0.2, 0.2], [0.7, 0.7, 0.7], 1, 'screen')[0]).toBeCloseTo(0.76);
  });
  it('does not paint black over LEDs outside the brush map', () => {
    const p = structuredClone(data[0]) as Preset;
    p.layers = [
      {
        id: 'paint',
        effect: 'paint',
        params: { colorsByLed: { 'pad.top.1': '#ff0000' } },
        opacity: 1,
        zone: 'all',
        enabled: true,
        blend: 'normal',
      },
      { ...p.layers[0], effect: 'static', params: { color: '#00ff00' } },
    ];
    const f = renderPreview(p, layout as Layout, 0, 1);
    expect(f[0][0]).toBeGreaterThan(0);
    expect(f[0][1]).toBe(0);
    expect(f[1][1]).toBeGreaterThan(0);
  });
  it('generates safe 7-bit frames for every bundled preset', () => {
    for (const p of data)
      for (const time of [0, 1, 1000]) {
        const frame = renderPreview(p as Preset, layout as Layout, time, 1);
        expect(frame).toHaveLength(layout.leds.length);
        expect(frame.flat().every((n) => Number.isInteger(n) && n >= 0 && n <= 127)).toBe(true);
      }
  });
  it('migrates old presets and rejects future or unsafe files', () => {
    const p = { ...data[0], schema: 0, post: undefined };
    expect(validatePreset(p).schema).toBe(1);
    expect(() => validatePreset({ ...data[0], schema: 100 })).toThrow();
    expect(() => validatePreset({ ...data[0], id: '../escape' })).toThrow();
    expect(() => validatePreset({ ...data[0], post: { ...data[0].post, gamma: 0 } })).toThrow();
  });
  it('dithers without wrapping error diffusion across rows', () => {
    const pixels = new Uint8ClampedArray(4 * 8);
    for (let i = 0; i < 8; i++)
      pixels.set([i % 2 ? 255 : 0, i % 2 ? 255 : 0, i % 2 ? 255 : 0, 255], i * 4);
    expect(ditherImage(pixels, 4, 2)).toEqual([0, 1, 0, 1, 0, 1, 0, 1]);
  });
});
