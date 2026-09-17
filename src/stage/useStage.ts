import { useEffect, useRef, useState } from 'react';
import { stageCommand, subscribeStage } from './api';
import { emptyStage, type Song, type StageSnapshot } from './types';
export function useStage(onError: (s: string) => void) {
  const [state, setState] = useState(emptyStage),
    [song, setSong] = useState<Song | null>(null);
  const frame = useRef({ state: emptyStage(), received: performance.now() }),
    last = useRef(0),
    revision = useRef(-1);
  useEffect(() => {
    let dead = false,
      received = false;
    let off: (() => void) | undefined;
    const accept = (s: StageSnapshot) => {
      if (dead) return;
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
  return { state, song, frame };
}
