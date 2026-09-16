import { useEffect, useState } from 'react';
import { getInputState, subscribe } from './api';
import { emptyInput, type InputState } from './input';
export function useInput() {
  const [state, setState] = useState(emptyInput);
  useEffect(() => {
    let dead = false,
      received = false;
    let cleanup: (() => void) | undefined;
    void subscribe<InputState>('input_state', (s) => {
      received = true;
      if (!dead) setState(s);
    }).then(async (off) => {
      if (dead) return off();
      cleanup = off;
      const current = await getInputState();
      if (!dead && !received) setState(current);
    });
    return () => {
      dead = true;
      cleanup?.();
    };
  }, []);
  return state;
}
