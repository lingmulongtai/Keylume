import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/ibm-plex-sans-jp/400.css';
import '@fontsource/ibm-plex-sans-jp/500.css';
import '@fontsource/ibm-plex-sans-jp/600.css';
import './styles.css';
import App from './App';
import { StageWindow } from './stage/Stage';
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {new URLSearchParams(location.search).get('view') === 'stage' ? <StageWindow /> : <App />}
  </StrictMode>,
);
