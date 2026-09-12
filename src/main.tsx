import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { LanguageProvider } from './context/LanguageContext';
import { TelemetryProvider } from './context/TelemetryContext';
import './index.css';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <LanguageProvider>
      <TelemetryProvider>
        <App />
      </TelemetryProvider>
    </LanguageProvider>
  </React.StrictMode>
);
