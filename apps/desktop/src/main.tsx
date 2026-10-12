import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { homeGreeting } from './homeGreetings';
import { LanguageProvider, resolveInitialLanguage } from './i18n-provider';
import { resolveLocale } from './i18n';
import './styles.css';
import './fonts';

let greetingStorage: Storage | undefined;
try { greetingStorage = window.localStorage; } catch { /* Optional persistence. */ }
// Outside render: StrictMode must not consume the once-per-date greeting twice.
const greeting = homeGreeting(new Date(), Math.random, greetingStorage);

// Resolve the initial interface language *before* mounting App so an English
// system preference never flashes a Chinese first paint. The greeting identity
// is unchanged by this; only the rendered text follows the resolved catalog.
void resolveInitialLanguage().then(({ setting, preferred }) => {
  document.documentElement.lang = resolveLocale(setting, preferred).locale;
  createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <LanguageProvider initialSetting={setting} initialPreferred={preferred}>
        <App greeting={greeting} />
      </LanguageProvider>
    </React.StrictMode>,
  );
});
