import type { AppState, Preset, Settings } from './types';
export interface ViewProps {
  state: AppState;
  act: (name: string, args?: Record<string, unknown>) => Promise<boolean>;
  edit: (preset: Preset) => void;
  saveSettings: (settings: Settings) => void;
  toast: (message: string) => void;
}
