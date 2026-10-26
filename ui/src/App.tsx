import { useTranslation } from 'react-i18next';

import { ScanScreen } from './features/scan/ScanScreen';
import './App.css';

function App() {
  const { t } = useTranslation();

  return (
    <main className="app">
      <h1>{t('app.title')}</h1>
      <p>{t('app.tagline')}</p>
      <ScanScreen />
    </main>
  );
}

export default App;
