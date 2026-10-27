import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './App';
import { ViewerApp } from './features/viewer/ViewerApp';
import './i18n';
import './index.css';

const root = document.getElementById('root');
if (!root) {
  throw new Error('Root element #root not found');
}

// The viewer window loads the same page with `?window=viewer`.
const isViewer = new URLSearchParams(window.location.search).get('window') === 'viewer';

createRoot(root).render(<StrictMode>{isViewer ? <ViewerApp /> : <App />}</StrictMode>);
