import current from '../resources/layout.json';
import legacy from '../resources/layout-v1.json';
import type { Layout } from './types';

function equalGeometry(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (!a || !b || typeof a !== 'object' || typeof b !== 'object') return false;
  const left = Object.entries(a),
    right = Object.entries(b);
  return (
    left.length === right.length &&
    left.every(
      ([key, value]) =>
        Object.hasOwn(b, key) && equalGeometry(value, (b as Record<string, unknown>)[key]),
    )
  );
}

export function upgradeLayout(layout: Layout): Layout {
  if (
    layout.geometryRevision ||
    layout.leds.length !== legacy.leds.length ||
    !equalGeometry(layout.canvas, legacy.canvas) ||
    !equalGeometry(layout.decor, legacy.decor) ||
    layout.leds.some((l) => {
      const old = legacy.leds.find((v) => v.id === l.id);
      return !old || !equalGeometry(l.pos, old.pos) || !equalGeometry(l.size, old.size);
    })
  )
    return layout;
  const updated = structuredClone(current) as Layout;
  for (const led of updated.leds) {
    const old = layout.leds.find((v) => v.id === led.id)!;
    led.address = structuredClone(old.address);
    led.kind = old.kind;
    led.group = old.group;
    led.label = old.label ?? led.label;
    led.verified = old.verified;
  }
  return updated;
}

export const keybed = (layout: Layout) =>
  layout.decor.keybed ?? { y: 239, h: 135, blackHeight: 84 };
export function keyPosition(layout: Layout, note: number) {
  const keys = layout.decor.keys;
  const key = keys.find((k) => k.note === Math.max(36, Math.min(96, note))) ?? keys[0];
  return (key.x + key.w / 2) / layout.canvas.w;
}
export const buttonLabel = (id: string) => current.leds.find((l) => l.id === id)?.label ?? id;
