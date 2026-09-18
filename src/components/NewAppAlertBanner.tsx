import React from 'react';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { Radio, Search, X } from 'lucide-react';

interface Props {
  onFilterApp?: (name: string) => void;
}

export const NewAppAlertBanner: React.FC<Props> = ({ onFilterApp }) => {
  const { t, isRtl } = useLanguage();
  const { newAppAlert, clearAlert } = useTelemetry();

  if (!newAppAlert) return null;

  const handleFilter = () => {
    if (onFilterApp) {
      onFilterApp(newAppAlert.name);
    }
    clearAlert();
  };

  return (
    <div className="fixed bottom-6 right-6 rtl:right-auto rtl:left-6 z-50 max-w-sm w-full animate-in slide-in-from-bottom-5 duration-300">
      <div className="bg-white dark:bg-slate-900/95 border border-transparent dark:border-brand-cyan/50 rounded-2xl p-3.5 shadow-2xl backdrop-blur-xl dark:shadow-brand-cyan/20 text-slate-900 dark:text-slate-100">
        {/* Header */}
        <div className="flex items-start justify-between gap-2.5 mb-2">
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-xl bg-cyan-500/10 dark:bg-brand-cyan/15 text-cyan-600 dark:text-brand-cyan shrink-0">
              <Radio className="w-4 h-4 animate-pulse" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-slate-900 dark:text-white">
                {isRtl ? 'تم رصد تطبيق جديد يتصل بالشبكة' : 'New Application Activity'}
              </h4>
              <p className="text-xs text-slate-700 dark:text-slate-300 font-semibold mt-0.5">
                <span className="text-cyan-600 dark:text-brand-cyan font-bold">{newAppAlert.name}</span>{' '}
                <span className="text-[10px] text-slate-500 dark:text-slate-400 font-mono">(PID {newAppAlert.pid})</span>
              </p>
            </div>
          </div>
          <button
            onClick={clearAlert}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 transition"
            title={isRtl ? 'إغلاق الإشعار' : 'Dismiss'}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Remote details */}
        <div className="bg-slate-50 dark:bg-slate-950/80 border border-transparent dark:border-slate-800 rounded-xl p-2 text-[11px] font-mono text-slate-700 dark:text-slate-300 mb-3 shadow-inner dark:shadow-none">
          <div className="flex justify-between items-center text-slate-500 dark:text-slate-400">
            <span>{isRtl ? 'الجهة:' : 'Target:'}</span>
            <span className="text-slate-800 dark:text-slate-200 font-semibold">{newAppAlert.remote_ip}:{newAppAlert.remote_port} ({newAppAlert.protocol})</span>
          </div>
        </div>

        {/* Action Button */}
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={clearAlert}
            className="px-3 py-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-medium transition"
          >
            {isRtl ? 'تجاهل' : 'Dismiss'}
          </button>
          <button
            onClick={handleFilter}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-700 dark:bg-brand-cyan dark:hover:bg-cyan-400 text-white dark:text-slate-950 font-bold text-xs shadow-md shadow-brand-cyan/20 transition"
          >
            <Search className="w-3.5 h-3.5" />
            <span>{isRtl ? 'عرض في الجدول' : 'View in Table'}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
