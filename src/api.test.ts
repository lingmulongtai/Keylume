import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import presets from '../resources/presets.json';

beforeEach(() => {
  vi.resetModules();
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
});
afterEach(() => vi.unstubAllGlobals());

it('restores active preset, coexist mode, and pause status together', async () => {
  const api = await import('./api');
  await api.command('apply_preset', { id: presets[1].id });
  await api.command('set_coexist_mode', { mode: 'handoff' });
  await api.command('set_paused', { paused: true });
  vi.resetModules();
  const restored = await (await import('./api')).getState();
  expect(restored.status.activePreset).toBe(presets[1].id);
  expect(restored.status.effectiveMode).toBe('handoff');
  expect(restored.status.connection).toBe('paused');
});

it('replaces every active-preset reference when deleting the active custom preset', async () => {
  const api = await import('./api');
  await api.command('save_preset', { preset: { ...presets[0], id: 'custom', builtin: false } });
  await api.command('delete_preset', { id: 'custom' });
  vi.resetModules();
  const restored = await (await import('./api')).getState();
  expect(restored.preset.id).toBe(presets[0].id);
  expect(restored.settings.activePreset).toBe(restored.preset.id);
  expect(restored.status.activePreset).toBe(restored.preset.id);
});

it('keeps imported and colliding user presets when restoring preview storage', async () => {
  const api = await import('./api');
  await api.command('import_preset', { json: JSON.stringify(presets[0]) });
  const state = await api.getState();
  const imported = state.presets.find((p) => p.id !== presets[0].id && p.name === presets[0].name)!;
  expect(imported.builtin).toBe(false);
  const saved = JSON.parse(localStorage.getItem('keylume-preview-v1')!);
  saved.presets.push({ ...presets.at(-1), name: 'My existing preset', builtin: false });
  saved.presets.push({ ...presets[0], id: 'legacy-import', builtin: true });
  localStorage.setItem('keylume-preview-v1', JSON.stringify(saved));
  vi.resetModules();
  const restored = await (await import('./api')).getState();
  expect(restored.presets.find((p) => p.id === presets.at(-1)!.id)?.name).toBe(
    'My existing preset',
  );
  expect(restored.presets.find((p) => p.id === 'legacy-import')?.builtin).toBe(false);
  expect(restored.presets.some((p) => p.id === imported.id)).toBe(true);
});
