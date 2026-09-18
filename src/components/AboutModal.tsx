import React, { useEffect } from 'react';
import { ShieldCheck, X, Globe, ExternalLink, Cpu, Code2, Users, Layers, Award } from 'lucide-react';
import { useLanguage } from '../context/LanguageContext';
import { openExternalLink } from '../utils/navigation';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutModal: React.FC<Props> = ({ isOpen, onClose }) => {
  const { isRtl } = useLanguage();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
    }
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-slate-950/80 backdrop-blur-md p-4 animate-fade-in"
      onClick={onClose}
    >
      <div
        className="bg-slate-900/95 border border-slate-700/80 rounded-2xl max-w-xl w-full p-6 shadow-2xl relative text-slate-100 overflow-hidden ring-1 ring-white/10"
        onClick={(e) => e.stopPropagation()}
        dir={isRtl ? 'rtl' : 'ltr'}
      >
        {/* Glow ambient accent */}
        <div className="absolute -top-24 -right-24 w-48 h-48 bg-brand-cyan/20 rounded-full blur-3xl pointer-events-none" />
        <div className="absolute -bottom-24 -left-24 w-48 h-48 bg-brand-purple/20 rounded-full blur-3xl pointer-events-none" />

        {/* Close Button */}
        <button
          onClick={onClose}
          className="absolute top-4 left-4 rtl:left-4 rtl:right-auto ltr:right-4 ltr:left-auto p-2 rounded-xl bg-slate-800/80 hover:bg-slate-700 border border-slate-700 text-slate-400 hover:text-white transition shadow-sm"
          title={isRtl ? 'إغلاق' : 'Close'}
        >
          <X className="w-4 h-4" />
        </button>

        {/* Header */}
        <div className="flex items-center gap-3.5 mb-5">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-tr from-brand-emerald via-brand-cyan to-brand-purple p-[2px] shadow-lg shadow-brand-cyan/20 shrink-0">
            <div className="w-full h-full bg-slate-950 rounded-[10px] flex items-center justify-center overflow-hidden p-1">
              <img src="/logo.png" alt="Flux" className="w-full h-full object-contain" />
            </div>
          </div>
          <div>
            <h2 className="text-xl font-bold tracking-tight text-white flex items-center gap-2">
              <span>Flux</span>
              <span className="text-xs font-mono font-normal px-2 py-0.5 rounded-full bg-brand-cyan/15 text-brand-cyan border border-brand-cyan/30">
                v1.0.0
              </span>
            </h2>
            <p className="text-xs text-slate-400">
              {isRtl
                ? 'متحكم النطاق الترددي المتقدم وتشكيل حزم البيانات لنظام ويندوز'
                : 'Advanced Windows Kernel Bandwidth Controller & Traffic Shaper'}
            </p>
          </div>
        </div>

        {/* Developer & Team Showcase Banner */}
        <div className="bg-gradient-to-br from-slate-950/90 to-slate-900/90 border border-brand-cyan/30 rounded-xl p-4 mb-4 shadow-inner space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-2.5">
              <div className="p-2 rounded-lg bg-brand-cyan/10 text-brand-cyan">
                <Code2 className="w-5 h-5" />
              </div>
              <div>
                <span className="text-[11px] font-medium text-slate-400 block">
                  {isRtl ? 'المطور وهندسة النواة:' : 'Lead Engineer & Architect:'}
                </span>
                <span className="text-sm font-bold text-white font-sans">
                  {isRtl ? 'المهندس أمجد علوان' : 'Eng. Amjad Alwan'}
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2.5">
              <div className="p-1 rounded-lg bg-brand-purple/10 border border-brand-purple/20 flex items-center justify-center">
                <img src="/tubba-soft-logo.png" alt="Tubba Soft" className="w-7 h-7 object-contain" />
              </div>
              <div>
                <span className="text-[11px] font-medium text-slate-400 block">
                  {isRtl ? 'فريق التطوير والأنظمة:' : 'Development Team:'}
                </span>
                <span className="text-sm font-bold text-slate-200">
                  {isRtl ? 'فريق تبع سوفت للحلول البرمجية' : 'Tubba Soft Engineering Team'}
                </span>
              </div>
            </div>
          </div>

          <div className="pt-2 border-t border-slate-800 flex items-center justify-between text-xs">
            <div className="flex items-center gap-2 text-slate-400">
              <Globe className="w-3.5 h-3.5 text-brand-cyan" />
              <span>{isRtl ? 'الموقع الرسمي المعتمد:' : 'Official Platform:'}</span>
            </div>
            <button
              onClick={() => openExternalLink('https://tubbasoft.com')}
              className="flex items-center gap-1.5 text-brand-cyan font-mono hover:underline font-semibold cursor-pointer group"
            >
              <span>tubbasoft.com</span>
              <ExternalLink className="w-3 h-3 group-hover:translate-x-0.5 transition-transform" />
            </button>
          </div>
        </div>

        {/* Technical Architecture Specs */}
        <div className="bg-slate-950/70 p-3.5 rounded-xl border border-slate-800 text-xs font-mono text-slate-300 space-y-1.5 mb-5">
          <div className="flex items-center justify-between">
            <span className="text-slate-500">* Kernel Driver:</span>
            <span className="text-emerald-400 font-semibold">WinDivert64.sys (v2.2.2-A Official)</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-slate-500">* Engine Mode:</span>
            <span className="text-cyan-300">Kernel Packet Diversion & Token Bucket Shaper</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-slate-500">* Telemetry Stream:</span>
            <span className="text-purple-300">Reactive IP-Helper + Atomic Wire Counters (300ms)</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-slate-500">* Filter Rule:</span>
            <span className="text-slate-400">!loopback and (tcp or udp)</span>
          </div>
        </div>

        {/* Action Button */}
        <div className="flex items-center justify-end">
          <button
            onClick={onClose}
            className="px-5 py-2 rounded-xl bg-brand-cyan/20 hover:bg-brand-cyan/30 text-brand-cyan font-semibold transition text-xs border border-brand-cyan/40 shadow-sm"
          >
            {isRtl ? 'حسناً، فهمت' : 'Got it'}
          </button>
        </div>
      </div>
    </div>
  );
};
