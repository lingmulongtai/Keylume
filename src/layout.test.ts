import { describe, expect, it } from 'vitest';
import current from '../resources/layout.json';
import legacy from '../resources/layout-v1.json';
import { keyPosition, upgradeLayout } from './layout';
import { validateLayout } from './api';
import type { Layout } from './types';

describe('Launchkey MK4 61 physical layout', () => {
  it('places 61 keys below the left wheels/faders, central display and right encoders/pads', () => {
    const l = validateLayout(current);
    expect(l.canvas.w / l.canvas.h).toBeCloseTo(895 / 264, 2);
    expect(l.decor.keys.filter((k) => !k.black)).toHaveLength(36);
    expect(l.decor.keys.filter((k) => k.black)).toHaveLength(25);
    expect(new Set(l.decor.keys.map((k) => k.note)).size).toBe(61);
    expect(Math.max(...l.decor.faders.map((f) => f.x))).toBeLessThan(l.decor.display.x);
    expect(l.decor.display.x + l.decor.display.w).toBeLessThan(l.leds[0].pos.x);
    expect(l.decor.encoders[0].y).toBeLessThan(l.leds[0].pos.y);
    for (const wheel of l.decor.wheels) expect(wheel.y + wheel.h).toBeLessThan(l.decor.keybed!.y);
    const button = (id: string) => l.leds.find((b) => b.id === `btn.${id}`)!;
    expect(button('stop').pos.x).toBe(button('play').pos.x);
    expect(button('loop').pos.x).toBe(button('record').pos.x);
    expect(button('stop').pos.y).toBeLessThan(button('play').pos.y);
    expect(button('loop').pos.y).toBeLessThan(button('record').pos.y);
    expect(keyPosition(l, 36)).toBeLessThan(0.05);
    expect(keyPosition(l, 96)).toBeGreaterThan(0.95);
    expect(keyPosition(l, 0)).toBe(keyPosition(l, 36));
  });

  it('upgrades only unchanged legacy geometry and preserves calibration', () => {
    const old = structuredClone(legacy) as Layout;
    old.leds[20].verified = true;
    old.leds[20].address.sysexId = 65;
    old.leds[20].kind = 'none';
    old.leds[20].group = 'buttons';
    old.canvas = { h: old.canvas.h, w: old.canvas.w };
    const updated = upgradeLayout(old);
    expect(updated.geometryRevision).toBe(2);
    expect(updated.leds[20].group).toBe('buttons');
    expect(updated.leds.map((l) => [l.id, l.address, l.kind, l.verified])).toEqual(
      old.leds.map((l) => [l.id, l.address, l.kind, l.verified]),
    );
    expect(upgradeLayout(updated)).toBe(updated);
    old.leds[0].pos.x += 1;
    expect(upgradeLayout(old)).toBe(old);
    expect(() =>
      validateLayout({ ...current, decor: { ...current.decor, controls: [{}] } }),
    ).toThrow();
  });
});
