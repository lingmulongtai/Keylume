import { expect, test } from 'vitest';
import { activeBindings, defaultController } from './controller';

test('native feature actions need opt-in while ordinary button mappings stay active', () => {
  const c = defaultController();
  expect(c.performance.scale).toBeUndefined();
  c.performance.scale = { action: 'lighting', value: '1' };
  expect(activeBindings(c).scale).toBeUndefined();
  expect(activeBindings(c)['btn.trackNext']).toEqual({ action: 'lighting', value: '1' });
  expect(c.performance.scale.action).toBe('lighting');
  c.nativeFeatures = true;
  expect(activeBindings(c).scale).toEqual(c.performance.scale);
});
