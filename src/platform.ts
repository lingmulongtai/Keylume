export const isMac = /Mac/i.test(globalThis.navigator?.platform ?? '');
export const systemName = isMac ? 'Mac' : 'Windows';
export const trayName = isMac ? 'メニューバー' : 'トレイ';
export const undoShortcut = isMac ? 'Cmd+Z' : 'Ctrl+Z';
