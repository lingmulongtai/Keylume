import { describe, it, expect } from 'vitest';
import { calibrationHandle, keys, union } from './geometry';
import { defaultStageSettings } from './types';
describe('stage geometry', () => {
  it('grabs the closest calibration line in screen pixels without a wide height dead zone', () => {
    const desktop = { x: -2560, y: 0, width: 5120, height: 1440 };
    const view = { desktop, monitor: { ...desktop, id: 0, name: 'preview', scale: 1 } };
    const s = defaultStageSettings;
    expect(calibrationHandle(51, 405, s, view, 1000, 500)).toBe('left');
    expect(calibrationHandle(949, 400, s, view, 1000, 500)).toBe('right');
    expect(calibrationHandle(500, 411, s, view, 1000, 500)).toBe('lineY');
    expect(calibrationHandle(500, 250, s, view, 1000, 500)).toBeNull();
    const right = { ...view, monitor: { ...view.monitor, x: 0, width: 2560 } };
    expect(calibrationHandle(900, 200, s, right, 1000, 500)).toBe('right');
  });
  it('aligns 61 keys with black keys straddling white boundaries', () => {
    const k = keys(36, 96);
    expect(k).toHaveLength(61);
    expect(k.filter((n) => !n.black)).toHaveLength(36);
    expect(k[0].x).toBe(0);
    expect(k.at(-1)!.x + k.at(-1)!.width).toBeCloseTo(1);
    expect(k[1].x).toBeLessThan(k[2].x);
    expect(k[1].x + k[1].width).toBeGreaterThan(k[2].x);
  });
  it('keeps negative coordinates and unlike display sizes in one physical desktop', () => {
    expect(
      union([
        { x: -2560, y: 0, width: 2560, height: 1440 },
        { x: 0, y: 0, width: 1920, height: 1080 },
      ]),
    ).toEqual({ x: -2560, y: 0, width: 4480, height: 1440 });
  });
});
