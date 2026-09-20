import { expect, it } from 'vitest';
import { settingsDiff, mergeSettings } from './settings-patch';
it('preserves simultaneous hardware changes while applying queued UI changes', () => {
  const base = {
    piano: { volume: 0.5, effects: { reverb: 0, delay: 0 } },
    controller: { mode: 'performance' },
  };
  const ui = { ...base, piano: { ...base.piano, volume: 0.7 } };
  const patch = settingsDiff(base, ui);
  expect(patch).toEqual({ piano: { volume: 0.7 } });
  const hardware = { ...base, piano: { ...base.piano, effects: { reverb: 0.9, delay: 0 } } };
  const merged = mergeSettings(hardware, patch);
  expect(merged.piano).toEqual({ volume: 0.7, effects: { reverb: 0.9, delay: 0 } });
  const queued = mergeSettings(patch, { controller: { mode: 'desktop' } });
  expect(mergeSettings(merged, queued).controller.mode).toBe('desktop');
});
