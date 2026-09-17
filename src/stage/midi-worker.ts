import { parseMidi } from './midi';
self.onmessage = ({ data }: { data: { bytes: ArrayBuffer; title: string } }) => {
  try {
    self.postMessage({ song: parseMidi(data.bytes, data.title) });
  } catch (error) {
    self.postMessage({ error: String(error instanceof Error ? error.message : error) });
  }
};
