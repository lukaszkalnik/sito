import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './App';
import { isViewerWindow } from './features/viewer/api';
import { ViewerApp } from './features/viewer/ViewerApp';
import './i18n';
import './index.css';

const root = document.getElementById('root');
if (!root) {
  throw new Error('Root element #root not found');
}

// The viewer window loads the same page.
createRoot(root).render(<StrictMode>{isViewerWindow() ? <ViewerApp /> : <App />}</StrictMode>);
