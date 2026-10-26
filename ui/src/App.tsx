import { useTranslation } from 'react-i18next';

import './App.css';

function App() {
  const { t } = useTranslation();

  return (
    <main className="app">
      <h1>{t('app.title')}</h1>
      <p>{t('app.tagline')}</p>
      <button type="button" disabled>
        {t('scan.chooseFolder')}
      </button>
    </main>
  );
}

export default App;
