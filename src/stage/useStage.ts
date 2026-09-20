import { useCallback, useEffect, useRef, useState } from 'react';
import { stageCommand, subscribeStage } from './api';
import { emptyStage, type Song, type StageSnapshot, type StageSettings } from './types';
export function useStage(onError: (s: string) => void) {
  const [state, setState] = useState(emptyStage),
    [song, setSong] = useState<Song | null>(null);
  const frame = useRef({ state: emptyStage(), received: performance.now() }),
    last = useRef(0),
    revision = useRef(-1);
  const overrides = useRef<Partial<StageSettings>>({});
  const optimistic = useCallback((patch: Partial<StageSettings>) => {
    overrides.current = { ...overrides.current, ...patch };
    frame.current = {
      ...frame.current,
      state: { ...frame.current.state, settings: { ...frame.current.state.settings, ...patch } },
    };
    setState((s) => ({ ...s, settings: { ...s.settings, ...patch } }));
  }, []);
  const settled = useCallback((patch: Partial<StageSettings>) => {
    for (const key of Object.keys(patch) as (keyof StageSettings)[])
      if (overrides.current[key] === patch[key]) delete overrides.current[key];
  }, []);
  useEffect(() => {
    let dead = false,
      received = false;
    let off: (() => void) | undefined;
    const accept = (s: StageSnapshot) => {
      if (dead) return;
      s = { ...s, settings: { ...s.settings, ...overrides.current } };
      received = true;
      frame.current = { state: s, received: performance.now() };
      if (performance.now() - last.current > 100 || revision.current !== s.songRevision) {
        setState(s);
        last.current = performance.now();
      }
      if (revision.current !== s.songRevision) {
        revision.current = s.songRevision;
        const r = s.songRevision;
        void stageCommand<Song | null>('song')
          .then((song) => {
            if (!dead && revision.current === r) setSong(song);
          })
          .catch((e) => onError(String(e)));
      }
    };
    void subscribeStage(accept)
      .then(async (unsubscribe) => {
        if (dead) return unsubscribe();
        off = unsubscribe;
        const s = await stageCommand('state');
        if (!received) accept(s);
      })
      .catch((e) => onError(String(e)));
    return () => {
      dead = true;
      off?.();
      revision.current = -1;
    };
  }, [onError]);
  return { state, song, frame, optimistic, settled };
}
