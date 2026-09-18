import React from 'react';
import { AlertTriangle, ExternalLink, Power, ShieldAlert, Globe } from 'lucide-react';
import { useLanguage } from '../context/LanguageContext';
import { openExternalLink } from '../utils/navigation';

import { RemoteLockInfo } from '../context/TelemetryContext';

export type { RemoteLockInfo };

interface Props {
  lockInfo: RemoteLockInfo;
  onExitApp: () => void;
}

export const RemoteLockModal: React.FC<Props> = ({ lockInfo, onExitApp }) => {
  const { isRtl, t } = useLanguage();

  if (!lockInfo.isLocked) return null;

  const title = isRtl
    ? (lockInfo.titleAr || lockInfo.title || t.remote_locked_title)
    : (lockInfo.titleEn || lockInfo.title || 'Developer Notice');

  const message = isRtl
    ? (lockInfo.messageAr || lockInfo.message || t.remote_locked_default_msg)
    : (lockInfo.messageEn || lockInfo.message || 'This version has been suspended by the developer.');

  const actionLabel = isRtl
    ? (lockInfo.actionLabelAr || lockInfo.actionLabel || t.remote_locked_action)
    : (lockInfo.actionLabelEn || lockInfo.actionLabel || 'Download Update Now');

  const handleAction = () => {
    if (lockInfo.actionUrl) {
      openExternalLink(lockInfo.actionUrl);
    } else {
      openExternalLink('https://tubbasoft.com');
    }
  };

  return (
    <div
      className="fixed inset-0 z-[9999] flex items-center justify-center bg-slate-950/95 backdrop-blur-xl p-4 select-none animate-fade-in"
      dir={isRtl ? 'rtl' : 'ltr'}
    >
      {/* Background ambient lighting */}
      <div className="absolute -top-32 -right-32 w-96 h-96 bg-red-500/10 rounded-full blur-3xl pointer-events-none" />
      <div className="absolute -bottom-32 -left-32 w-96 h-96 bg-brand-purple/15 rounded-full blur-3xl pointer-events-none" />

      <div className="bg-slate-900/95 border border-red-500/30 rounded-3xl max-w-lg w-full p-8 shadow-2xl relative text-slate-100 overflow-hidden ring-1 ring-red-500/20 text-center">
        {/* Top Glow bar */}
        <div className="absolute top-0 inset-x-0 h-1 bg-gradient-to-r from-red-500 via-brand-purple to-brand-cyan" />

        {/* Tubba Soft Brand Header */}
        <div className="flex items-center justify-center gap-3 mb-6">
          <img
            src="/tubba-soft-logo.png"
            alt="Tubba Soft"
            className="w-12 h-12 object-contain filter drop-shadow-[0_0_12px_rgba(168,85,247,0.4)]"
          />
          <div className="text-start">
            <span className="text-xs font-bold tracking-widest text-slate-400 uppercase block">
              Tubba Soft Systems
            </span>
            <span className="text-lg font-black text-white tracking-tight">
              Flux Control Gate
            </span>
          </div>
        </div>

        {/* Warning Icon Pill */}
        <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-red-500/15 border border-red-500/30 text-red-400 text-xs font-semibold mb-4">
          <ShieldAlert className="w-4 h-4 text-red-400" />
          <span>{isRtl ? 'إشعار إداري معتمد من المطور' : 'Official Developer Notice'}</span>
        </div>

        {/* Title */}
        <h2 className="text-2xl font-black text-white tracking-tight mb-3">
          {title}
        </h2>

        {/* Message Card */}
        <div className="bg-slate-950/80 border border-slate-800/90 rounded-2xl p-5 mb-6 text-slate-300 text-sm leading-relaxed text-start shadow-inner">
          <p>{message}</p>
          <div className="mt-3 pt-3 border-t border-slate-800/80 flex items-center justify-between text-xs text-slate-400">
            <span>{isRtl ? 'حالة درايفر النواة:' : 'Kernel Status:'}</span>
            <span className="text-emerald-400 font-semibold">
              {isRtl ? 'تم فك الربط بأمان (الإنترنت حر)' : 'Safely Unhooked (Network Intact)'}
            </span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-col gap-3">
          <button
            onClick={handleAction}
            className="w-full py-3.5 px-6 rounded-xl bg-gradient-to-r from-brand-cyan to-brand-purple hover:brightness-110 text-white font-bold text-sm flex items-center justify-center gap-2 shadow-lg shadow-brand-cyan/20 hover:shadow-brand-cyan/30 transition cursor-pointer"
          >
            <span>{actionLabel}</span>
            <ExternalLink className="w-4 h-4" />
          </button>

          <div className="flex items-center gap-3">
            <button
              onClick={() => openExternalLink('https://tubbasoft.com')}
              className="flex-1 py-2.5 px-4 rounded-xl bg-slate-800/80 hover:bg-slate-700/80 border border-slate-700 text-slate-300 hover:text-white font-medium text-xs flex items-center justify-center gap-2 transition cursor-pointer"
            >
              <Globe className="w-3.5 h-3.5 text-brand-cyan" />
              <span>tubbasoft.com</span>
            </button>

            <button
              onClick={onExitApp}
              className="flex-1 py-2.5 px-4 rounded-xl bg-red-950/40 hover:bg-red-900/60 border border-red-500/30 text-red-300 font-medium text-xs flex items-center justify-center gap-2 transition cursor-pointer"
            >
              <Power className="w-3.5 h-3.5" />
              <span>{t.remote_locked_exit}</span>
            </button>
          </div>
        </div>

        {/* Footer Note */}
        <p className="mt-6 text-[11px] text-slate-400">
          {isRtl
            ? 'حقوق التطوير محفوظة لشركة تبع سوفت © Tubba Soft'
            : 'Engineered & Managed by Tubba Soft Solutions ©'}
        </p>
      </div>
    </div>
  );
};
