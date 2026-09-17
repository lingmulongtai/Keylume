import { describe, it, expect } from 'vitest';
import { keys, union } from './geometry';
describe('stage geometry', () => {
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
