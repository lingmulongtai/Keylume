type ObjectValue = Record<string, unknown>;
const object = (value: unknown): value is ObjectValue =>
  !!value && typeof value === 'object' && !Array.isArray(value);
export function settingsDiff(before: unknown, after: unknown): ObjectValue {
  if (!object(before) || !object(after)) return {};
  const patch: ObjectValue = {};
  for (const [key, value] of Object.entries(after)) {
    if (JSON.stringify(before[key]) === JSON.stringify(value)) continue;
    patch[key] = object(value) && object(before[key]) ? settingsDiff(before[key], value) : value;
  }
  return patch;
}
export function mergeSettings<T>(before: T, patch: unknown): T {
  if (!object(before) || !object(patch)) return patch as T;
  const next: ObjectValue = { ...before };
  for (const [key, value] of Object.entries(patch)) {
    if (['__proto__', 'prototype', 'constructor'].includes(key)) continue;
    next[key] = object(value) && object(next[key]) ? mergeSettings(next[key], value) : value;
  }
  return next as T;
}
