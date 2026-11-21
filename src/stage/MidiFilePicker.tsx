import { useRef, useState } from 'react';
import { Upload } from 'lucide-react';
import { Modal } from '../components';
import { midiFileTitle } from './import-midi';

export default function MidiFilePicker({
  label,
  inputLabel,
  disabled,
  onImport,
}: {
  label: string;
  inputLabel: string;
  disabled: boolean;
  onImport: (file: File, title: string) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState('');
  return (
    <>
      <button disabled={disabled} onClick={() => input.current?.click()}>
        <Upload size={15} />
        {label}
      </button>
      <input
        ref={input}
        type="file"
        hidden
        accept=".mid,.midi"
        aria-label={inputLabel}
        onChange={(e) => {
          const selected = e.target.files?.[0];
          e.target.value = '';
          if (selected) {
            setTitle(midiFileTitle(selected.name));
            setFile(selected);
          }
        }}
      />
      {file && (
        <Modal title="MIDIの曲名" onClose={() => setFile(null)}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              const name = title.trim();
              if (name && !disabled) {
                setFile(null);
                onImport(file, name);
              }
            }}
          >
            <label className="field">
              <span>曲名</span>
              <input
                autoFocus
                type="text"
                maxLength={200}
                value={title}
                onFocus={(e) => e.target.select()}
                onChange={(e) => setTitle(e.target.value)}
              />
            </label>
            <div className="modal-actions">
              <button type="button" onClick={() => setFile(null)}>
                キャンセル
              </button>
              <button className="primary" type="submit" disabled={disabled || !title.trim()}>
                読み込む
              </button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}
