import { describe, it, expect } from 'vitest';
import { compose, renderPreview, ditherImage } from './preview';
import { validatePreset, validateLayout } from './api';
import data from '../resources/presets.json';
import layout from '../resources/layout.json';
import type { Preset, Layout } from './types';
describe('preview and interchange', () => {
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
