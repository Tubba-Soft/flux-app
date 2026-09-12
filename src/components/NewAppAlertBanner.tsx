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
      <div className="bg-slate-900/95 border border-brand-cyan/50 rounded-2xl p-3.5 shadow-2xl backdrop-blur-xl shadow-brand-cyan/20">
        {/* Header */}
        <div className="flex items-start justify-between gap-2.5 mb-2">
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-xl bg-brand-cyan/15 text-brand-cyan shrink-0">
              <Radio className="w-4 h-4 animate-pulse" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-white">
                {isRtl ? 'تم رصد تطبيق جديد يتصل بالشبكة' : 'New Application Activity'}
              </h4>
              <p className="text-xs text-slate-300 font-semibold mt-0.5">
                <span className="text-brand-cyan font-bold">{newAppAlert.name}</span>{' '}
                <span className="text-[10px] text-slate-400 font-mono">(PID {newAppAlert.pid})</span>
              </p>
            </div>
          </div>
          <button
            onClick={clearAlert}
            className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition"
            title={isRtl ? 'إغلاق الإشعار' : 'Dismiss'}
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Remote details */}
        <div className="bg-slate-950/80 border border-slate-800 rounded-xl p-2 text-[11px] font-mono text-slate-300 mb-3">
          <div className="flex justify-between items-center text-slate-400">
            <span>{isRtl ? 'الجهة:' : 'Target:'}</span>
            <span className="text-slate-200">{newAppAlert.remote_ip}:{newAppAlert.remote_port} ({newAppAlert.protocol})</span>
          </div>
        </div>

        {/* Action Button */}
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={clearAlert}
            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium transition"
          >
            {isRtl ? 'تجاهل' : 'Dismiss'}
          </button>
          <button
            onClick={handleFilter}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-brand-cyan hover:bg-cyan-400 text-slate-950 font-bold text-xs shadow-md shadow-brand-cyan/20 transition"
          >
            <Search className="w-3.5 h-3.5" />
            <span>{isRtl ? 'عرض في الجدول' : 'View in Table'}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
