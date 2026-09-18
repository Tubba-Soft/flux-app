import React, { useState } from 'react';
import { Header } from './components/Header';
import { ProcessTreeTable } from './components/ProcessTreeTable';
import { RuleModal } from './components/RuleModal';
import { ProcessDetailModal } from './components/ProcessDetailModal';
import { NewAppAlertBanner } from './components/NewAppAlertBanner';
import { ProcessTraffic, StreamTraffic } from './types';
import { useLanguage } from './context/LanguageContext';
import { useTelemetry } from './context/TelemetryContext';

import { TaskbarWidget } from './components/TaskbarWidget';
import { AboutModal } from './components/AboutModal';
import { WidgetSettingsModal } from './components/WidgetSettingsModal';
import { RemoteLockModal } from './components/RemoteLockModal';

export const AppContent: React.FC = () => {
  const { t, isRtl } = useLanguage();
  const { processes, isBrowserPreview, remoteLockInfo } = useTelemetry();

  const [ruleModalTarget, setRuleModalTarget] = useState<ProcessTraffic | StreamTraffic | null>(null);
  const [inspectProc, setInspectProc] = useState<ProcessTraffic | null>(null);
  const [showAboutModal, setShowAboutModal] = useState<boolean>(false);
  const [showWidgetSettingsModal, setShowWidgetSettingsModal] = useState<boolean>(false);
  const [searchFilter, setSearchFilter] = useState<string>('');

  const handleExitApp = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch {
      window.close();
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-dark-bg select-none text-slate-900 dark:text-slate-100 overflow-hidden font-sans">
      {isBrowserPreview && (
        <div className="bg-amber-500/15 border-b border-amber-500/30 px-4 py-2 text-xs text-amber-700 dark:text-amber-300 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="font-bold">⚠️ {isRtl ? 'وضع معاينة المتصفح:' : 'Browser Preview Mode:'}</span>
            <span>
              {isRtl
                ? 'أنت تتصفح الواجهة عبر متصفح الويب (بيانات توضيحية). لفحص كرت الشبكة الفعلي وتطبيق قواعد تحديد السرعة، شغّل تطبيق Flux كمسؤول.'
                : 'Viewing in web browser (simulated data). To capture real network packets and enforce shaping rules, run the Flux desktop app as Administrator.'}
            </span>
          </div>
        </div>
      )}

      {/* Top Banner & Telemetry Header */}
      <Header
        onOpenAbout={() => setShowAboutModal(true)}
        onOpenWidgetSettings={() => setShowWidgetSettingsModal(true)}
      />

      {/* Main Process & Stream Tree Table */}
      <ProcessTreeTable
        searchQuery={searchFilter}
        onSearchQueryChange={setSearchFilter}
        onOpenRuleModal={(target) => setRuleModalTarget(target)}
        onOpenInspectModal={(proc) => setInspectProc(proc)}
      />

      {/* Bottom Status Bar */}
      <footer className="bg-dark-surface/90 border-t border-dark-border/40 px-6 py-2 flex items-center justify-between text-[11px] text-slate-600 dark:text-slate-400 select-none">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-brand-emerald animate-pulse" />
            <span className="font-semibold text-slate-800 dark:text-slate-200">Flux Engine v1.0.0</span>
          </span>
          <span className="text-slate-300 dark:text-slate-700">|</span>
          <span>{processes.length} Processes Tracked</span>
        </div>

        <div className="flex items-center gap-4">
          <span>WinDivert 2.2-A Kernel Shaper Layer</span>
          <span className="text-slate-300 dark:text-slate-700">|</span>
          <span className="text-slate-500 dark:text-slate-400 font-mono">100-300ms Ticker</span>
        </div>
      </footer>

      {/* Modals & Alerts */}
      <RuleModal
        target={ruleModalTarget}
        onClose={() => setRuleModalTarget(null)}
      />

      <ProcessDetailModal
        process={inspectProc}
        onClose={() => setInspectProc(null)}
      />

      <NewAppAlertBanner
        onFilterApp={(name) => setSearchFilter(name)}
      />

      <AboutModal
        isOpen={showAboutModal}
        onClose={() => setShowAboutModal(false)}
      />

      <WidgetSettingsModal
        isOpen={showWidgetSettingsModal}
        onClose={() => setShowWidgetSettingsModal(false)}
      />

      {/* Remote Kill-Switch / Version Deprecation Modal */}
      <RemoteLockModal
        lockInfo={remoteLockInfo}
        onExitApp={handleExitApp}
      />
    </div>
  );
};

export default function App() {
  const isWidget = window.location.hash === '#widget';

  React.useEffect(() => {
    if (isWidget) {
      document.documentElement.classList.add('widget-mode');
      document.body.classList.add('widget-mode');
      const root = document.getElementById('root');
      if (root) root.classList.add('widget-mode');
    }
  }, [isWidget]);

  if (isWidget) {
    return <TaskbarWidget />;
  }
  return <AppContent />;
}
