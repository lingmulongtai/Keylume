import { useEffect, useState } from 'react';
import { getInputState, subscribe } from './api';
import { emptyInput, type InputState } from './input';
export function useInput() {
  const [state, setState] = useState(emptyInput);
  const [pulse, setPulse] = useState<string | null>(null);
  const pulseId = state.pulse?.[0],
    pulseSerial = state.pulse?.[1];
  useEffect(() => {
    if (!pulseId) return;
    setPulse(pulseId);
    const timer = setTimeout(() => setPulse(null), 180);
    return () => clearTimeout(timer);
  }, [pulseId, pulseSerial]);
  useEffect(() => {
    let dead = false,
      received = false;
    let cleanup: (() => void) | undefined;
    void subscribe<InputState>('input_state', (s) => {
      received = true;
      if (!dead) setState(s);
    })
      .then(async (off) => {
        if (dead) return off();
        cleanup = off;
        const current = await getInputState();
        if (!dead && !received) setState(current);
      })
      .catch(() => {
        if (!dead)
          setState((previous) => ({ ...previous, lastMessage: '入力状態を取得できません' }));
      });
    return () => {
      dead = true;
      cleanup?.();
    };
  }, []);
  return pulse ? { ...state, held: [...state.held, pulse] } : state;
}
