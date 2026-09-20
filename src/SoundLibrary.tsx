import { useCallback, useEffect, useRef, useState } from 'react';
import { Download, Star, Upload, Check, Search } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { native } from './api';
import {
  categoryNames,
  factorySounds,
  libraryCommand,
  type LibraryState,
  type SoundEntry,
} from './sounds';
import type { ViewProps } from './ui-state';

export default function SoundLibrary({
  state,
  saveSettings,
  toast,
}: Pick<ViewProps, 'state' | 'saveSettings' | 'toast'>) {
  const latest = useRef(state);
  latest.current = state;
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const [library, setLibrary] = useState<LibraryState>({
      entries: factorySounds,
      progress: { active: false, received: 0, total: 32319396, error: '' },
    }),
    [search, setSearch] = useState(''),
    [category, setCategory] = useState('all'),
    [busy, setBusy] = useState('');
  const refresh = useCallback(
    () =>
      libraryCommand('state')
        .then(setLibrary)
        .catch((e) => toast(String(e))),
    [toast],
  );
  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    if (!busy && !library.progress.active) return;
    const timer = setInterval(() => void refresh(), 250);
    return () => clearInterval(timer);
  }, [busy, library.progress.active, refresh]);
  const working = !!busy || library.progress.active;
  const select = (id: string) => {
    if (mounted.current) {
      const settings = latest.current.settings;
      saveSettings({ ...settings, piano: { ...settings.piano, sound: id } });
    }
  };
  const piano = state.settings.piano;
  const choose = async (entry: SoundEntry) => {
    setBusy(entry.id);
    try {
      if (!entry.installed) {
        await libraryCommand('download');
        await refresh();
      }
      select(entry.id);
    } catch (e) {
      toast(String(e));
    } finally {
      setBusy('');
    }
  };
  const importFont = async () => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: 'SoundFont 2', extensions: ['sf2'] }],
      });
      if (!path) return;
      setBusy('import');
      const ids = await libraryCommand<string[]>('import', { path });
      await refresh();
      setCategory('Imported');
      setSearch('');
      if (ids[0]) select(ids[0]);
      toast(`${ids.length}音色を追加しました`);
    } catch (e) {
      toast(String(e));
    } finally {
      setBusy('');
    }
  };
  const visible = library.entries.filter(
    (e) =>
      (category === 'all' ||
        (category === 'favorites' && piano.favorites.includes(e.id)) ||
        (category === 'installed' && e.installed) ||
        e.category === category) &&
      `${e.name} ${categoryNames[e.category] ?? e.category}`
        .toLowerCase()
        .includes(search.toLowerCase()),
  );
  return (
    <section className="sound-library" aria-label="音源ライブラリ">
      <div className="library-toolbar">
        <label className="library-search">
          <Search size={16} />
          <input
            aria-label="音源を検索"
            placeholder="音源を検索"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <button disabled={!native || working} onClick={() => void importFont()}>
          <Upload size={15} />
          SoundFontを追加
        </button>
        <button
          disabled={!native || working}
          onClick={() => {
            setBusy('repair');
            void libraryCommand('download')
              .then(refresh)
              .catch((e) => toast(String(e)))
              .finally(() => setBusy(''));
          }}
        >
          追加パックを確認・修復
        </button>
      </div>
      <div className="library-categories" aria-label="音源カテゴリ">
        {[
          ['all', 'すべて'],
          ['favorites', 'お気に入り'],
          ['installed', '使用可能'],
          ...Object.entries(categoryNames),
        ].map(([id, label]) => (
          <button
            key={id}
            aria-pressed={id === category}
            className={id === category ? 'active' : ''}
            onClick={() => setCategory(id)}
          >
            {label}
          </button>
        ))}
      </div>
      <p className="library-description">
        {visible.length}音色 · 追加音色は初回のみ約32MBを取得。以後はオフラインで演奏できます。
        <br />
        ノブ右の上下ボタンで使用可能な音色を切替。お気に入りがあればその順に切り替えます。
      </p>
      {working && (
        <div className="library-progress" role="status">
          <span>
            {busy === 'import' ? 'SoundFontを読み込み中…' : '音源を準備しています…'}{' '}
            {Math.round(library.progress.received / 1e6)} / 32 MB
          </span>
          <progress max={library.progress.total || 32319396} value={library.progress.received} />
          {busy !== 'import' && (
            <button onClick={() => void libraryCommand('cancel').catch((e) => toast(String(e)))}>
              取得を中止
            </button>
          )}
        </div>
      )}
      {library.progress.error && !busy && <p role="alert">{library.progress.error}</p>}
      <div className="sound-grid">
        {visible.map((entry, i) => {
          const favorite = piano.favorites.includes(entry.id),
            selected = piano.sound === entry.id;
          return (
            <article key={entry.id} className={`sound-card ${selected ? 'selected' : ''}`}>
              <button
                className="sound-select"
                aria-pressed={selected}
                disabled={working}
                onClick={() => void choose(entry)}
              >
                <div
                  className="sound-art"
                  aria-hidden="true"
                  style={
                    {
                      '--sound-hue': String((entry.program * 19 + entry.bank * 7) % 360),
                    } as React.CSSProperties
                  }
                >
                  <svg viewBox="0 0 240 64">
                    {Array.from({ length: 40 }, (_, j) => (
                      <line
                        key={j}
                        x1={j * 6 + 3}
                        x2={j * 6 + 3}
                        y1={
                          32 -
                          (4 +
                            24 *
                              Math.abs(
                                Math.sin(j * 0.48 + entry.program) *
                                  Math.cos(j * 0.11 + entry.bank),
                              ))
                        }
                        y2={
                          32 +
                          (4 +
                            24 *
                              Math.abs(
                                Math.sin(j * 0.48 + entry.program) *
                                  Math.cos(j * 0.11 + entry.bank),
                              ))
                        }
                      />
                    ))}
                  </svg>
                  <span>{String(i + 1).padStart(2, '0')}</span>
                </div>
                <strong>{entry.name}</strong>
                <small>
                  {categoryNames[entry.category]}
                  {entry.bank > 0 ? ` · Bank ${entry.bank}` : ''}
                </small>
                <span className="sound-status">
                  {selected ? (
                    <>
                      <Check size={13} />
                      選択中
                    </>
                  ) : entry.installed ? (
                    '使用可能'
                  ) : (
                    <>
                      <Download size={13} />
                      初回ダウンロード
                    </>
                  )}
                </span>
              </button>
              <button
                className="sound-favorite"
                aria-label={`${entry.name}をお気に入り${favorite ? 'から外す' : 'に追加'}`}
                aria-pressed={favorite}
                onClick={() =>
                  saveSettings({
                    ...state.settings,
                    piano: {
                      ...piano,
                      favorites: favorite
                        ? piano.favorites.filter((id) => id !== entry.id)
                        : [...piano.favorites, entry.id].slice(0, 128),
                    },
                  })
                }
              >
                <Star size={16} fill={favorite ? 'currentColor' : 'none'} />
              </button>
            </article>
          );
        })}
      </div>
      {!visible.length && <p>該当する音色がありません。</p>}
      <p className="library-credit">
        追加音源: GeneralUser GS 2.0.3 · S. Christian Collins / GeneralUser GS License
        v2.0。持ち込みはSF2（最大256MB）に対応。
      </p>
    </section>
  );
}
