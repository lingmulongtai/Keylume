import { useCallback, useRef } from 'react';
import { stageCommand } from './api';
import type { StageSettings } from './types';

// One in-flight write and one merged pending patch keep fast drags responsive.
export function useStageChange(onError: (message: string) => void) {
  const pending = useRef<Partial<StageSettings>>({}),
    sending = useRef(false);
  return useCallback(
    (patch: Partial<StageSettings>) => {
      pending.current = { ...pending.current, ...patch };
      const flush = async () => {
        if (sending.current) return;
        sending.current = true;
        while (Object.keys(pending.current).length) {
          const patch = pending.current;
          pending.current = {};
          try {
            await stageCommand('settings', { patch });
          } catch (e) {
            onError(String(e));
          }
        }
        sending.current = false;
      };
      void flush();
    },
    [onError],
  );
}
