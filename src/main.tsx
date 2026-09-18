import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ThemeProvider } from './context/ThemeContext';
import { LanguageProvider } from './context/LanguageContext';
import { TelemetryProvider } from './context/TelemetryContext';
import './index.css';

import { TaskbarWidget } from './components/TaskbarWidget';

const isWidget = window.location.hash === '#widget';

if (isWidget) {
  document.documentElement.classList.add('widget-mode');
  document.body.classList.add('widget-mode');
  const root = document.getElementById('root');
  if (root) root.classList.add('widget-mode');
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    {isWidget ? (
      <TaskbarWidget />
    ) : (
      <ThemeProvider>
        <LanguageProvider>
          <TelemetryProvider>
            <App />
          </TelemetryProvider>
        </LanguageProvider>
      </ThemeProvider>
    )}
  </React.StrictMode>
);
