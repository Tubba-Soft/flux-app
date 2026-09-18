import React, { useState } from 'react';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { formatSpeed, formatBytes } from '../utils/formatters';
import { openExternalLink } from '../utils/navigation';
import { ThemeToggle } from './ThemeToggle';
import { BandwidthSparkline } from './BandwidthSparkline';
import {
  Activity,
  ArrowDownCircle,
  ArrowUpCircle,
  ShieldCheck,
  ShieldAlert,
  Globe,
  Radio,
  Sliders,
  Info,
  Layers,
  Database,
  Gauge,
  Minimize2,
  Power,
} from 'lucide-react';

interface HeaderProps {
  onOpenAbout?: () => void;
  onOpenWidgetSettings?: () => void;
}

export const Header: React.FC<HeaderProps> = ({ onOpenAbout, onOpenWidgetSettings }) => {
  const { t, lang, toggleLang, isRtl } = useLanguage();
  const {
    driverStatus,
    globalDownSpeed,
    globalUpSpeed,
    globalTotalDown,
    globalTotalUp,
    todayTotalDown,
    todayTotalUp,
    todayTotalBytes,
    activeSocketsCount,
    processes,
    elevateAdmin,
    autostart,
    setAutostart,
    runInBackground,
    setRunInBackground,
    taskbarWidget,
    setTaskbarWidget,
  } = useTelemetry();

  const [showAvModal, setShowAvModal] = useState(false);

  const isKernelActive = driverStatus.mode === 'kernel_shaper' && driverStatus.is_driver_loaded;

  return (
    <header className="bg-white dark:bg-[#0C1220] border-b border-transparent dark:border-slate-800/80 px-6 py-3.5 shadow-sm dark:shadow-none select-none transition-colors">
      {/* Top row: Brand + Controls + Actions */}
      <div className="flex flex-wrap items-center justify-between gap-4 pb-3 border-b border-transparent dark:border-slate-800/60">
        {/* Brand */}
        <div className="flex items-center gap-3">
          <div className="relative flex items-center justify-center w-10 h-10 rounded-xl bg-gradient-to-tr from-brand-emerald via-brand-cyan to-brand-purple p-[1.5px] shadow-md shrink-0">
            <div className="w-full h-full bg-slate-100 dark:bg-[#090D16] rounded-[10px] flex items-center justify-center overflow-hidden p-1">
              <img src="/logo.png" alt="Flux" className="w-full h-full object-contain" />
            </div>
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-white">
              {t.app_name}
            </h1>
            <p className="text-xs text-slate-500 dark:text-slate-400 font-medium">
              {t.tagline}
            </p>
          </div>
        </div>

        {/* Action Buttons Toolbar */}
        <div className="flex items-center flex-wrap gap-2">
          {/* Driver Status Badge */}
          <div
            className={`flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-semibold border transition ${
              isKernelActive
                ? 'bg-emerald-50 text-emerald-700 border-transparent dark:bg-emerald-950/40 dark:text-emerald-400 dark:border-emerald-500/30'
                : 'bg-amber-50 text-amber-700 border-transparent dark:bg-amber-950/40 dark:text-amber-400 dark:border-amber-500/30'
            }`}
          >
            {isKernelActive ? (
              <>
                <ShieldCheck className="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
                <span>{t.status_kernel_active}</span>
                <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
              </>
            ) : (
              <>
                <ShieldAlert className="w-4 h-4 text-amber-600 dark:text-amber-400" />
                <span>{t.status_passive_monitor}</span>
                <button
                  onClick={elevateAdmin}
                  className="ml-1.5 px-2 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-900 dark:text-amber-300 font-bold underline transition"
                  title={t.status_admin_req}
                >
                  {t.status_elevate_btn}
                </button>
              </>
            )}
          </div>

          {/* Taskbar Speed Widget Toggle */}
          <button
            onClick={() => setTaskbarWidget(!taskbarWidget)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-xs font-medium transition ${
              taskbarWidget
                ? 'bg-emerald-50 text-emerald-700 border-transparent dark:bg-emerald-950/40 dark:text-emerald-300 dark:border-emerald-500/30 shadow-sm'
                : 'bg-slate-100 hover:bg-slate-200 text-slate-700 border-transparent dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:border-slate-700/60'
            }`}
            title={isRtl ? 'عرض ودجت شريط المهام في أسفل الشاشة بجانب الساعة والواي فاي والبطارية' : 'Show Taskbar Speed Widget near clock and battery'}
          >
            <Gauge className="w-3.5 h-3.5" />
            <span>{isRtl ? 'ودجت المهام' : 'Taskbar Widget'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${taskbarWidget ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400 dark:bg-slate-600'}`} />
          </button>

          {/* Widget Customization / Settings Trigger */}
          <button
            onClick={() => onOpenWidgetSettings?.()}
            className="p-2 rounded-xl bg-slate-100 hover:bg-slate-200 text-slate-600 hover:text-cyan-600 dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:hover:text-cyan-400 border border-transparent dark:border-slate-700/60 transition shadow-sm dark:shadow-none"
            title={isRtl ? 'تخصيص ودجت شريط المهام (تثبيت، شفافية، ألوان)' : 'Widget Customization (Docking, Transparency, Themes)'}
          >
            <Sliders className="w-3.5 h-3.5" />
          </button>

          {/* Background Running Toggle */}
          <button
            onClick={() => setRunInBackground(!runInBackground)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-xs font-medium transition ${
              runInBackground
                ? 'bg-cyan-50 text-cyan-700 border-transparent dark:bg-cyan-950/40 dark:text-cyan-300 dark:border-cyan-500/30 shadow-sm'
                : 'bg-slate-100 hover:bg-slate-200 text-slate-700 border-transparent dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:border-slate-700/60'
            }`}
            title={isRtl ? 'التشغيل في الخلفية بصينية النظام عند إغلاق النافذة' : 'Run in background (System Tray) when closed'}
          >
            <Minimize2 className="w-3.5 h-3.5" />
            <span>{isRtl ? 'بالخلفية' : 'Tray Mode'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${runInBackground ? 'bg-cyan-500' : 'bg-slate-400 dark:bg-slate-600'}`} />
          </button>

          {/* Windows Autostart Toggle */}
          <button
            onClick={() => setAutostart(!autostart)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-xs font-medium transition ${
              autostart
                ? 'bg-purple-50 text-purple-700 border-transparent dark:bg-purple-950/40 dark:text-purple-300 dark:border-purple-500/30 shadow-sm'
                : 'bg-slate-100 hover:bg-slate-200 text-slate-700 border-transparent dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:border-slate-700/60'
            }`}
            title={isRtl ? 'التشغيل التلقائي مع إقلاع نظام ويندوز' : 'Start automatically on Windows boot'}
          >
            <Power className="w-3.5 h-3.5" />
            <span>{isRtl ? 'بدء تلقائي' : 'Autostart'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${autostart ? 'bg-purple-500' : 'bg-slate-400 dark:bg-slate-600'}`} />
          </button>

          {/* Info / About Button */}
          <button
            onClick={() => onOpenAbout?.()}
            className="p-2 rounded-xl bg-slate-100 hover:bg-slate-200 text-slate-600 hover:text-slate-900 dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:hover:text-white border border-transparent dark:border-slate-700/60 transition shadow-sm dark:shadow-none"
            title={t.av_notice_title}
          >
            <Info className="w-4 h-4" />
          </button>

          {/* Tubba Soft Official Logo Badge with direct website link */}
          <button
            onClick={() => openExternalLink('https://tubbasoft.com')}
            className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-purple-50/80 hover:bg-purple-100 text-purple-800 dark:bg-purple-950/40 dark:hover:bg-purple-900/50 dark:text-purple-300 border border-transparent dark:border-purple-800/40 transition group cursor-pointer shadow-sm dark:shadow-none"
            title={isRtl ? 'زيارة منصة شركة تبع سوفت الرسمية (tubbasoft.com)' : 'Visit official Tubba Soft platform (tubbasoft.com)'}
          >
            <img
              src="/tubba-soft-logo.png"
              alt="Tubba Soft"
              className="w-4 h-4 object-contain group-hover:scale-110 transition-transform"
            />
            <span className="text-xs font-semibold hidden sm:inline">
              Tubba Soft
            </span>
          </button>

          {/* 3-State Theme Toggle (Light / Dark / Auto System) */}
          <ThemeToggle />

          {/* Language Switcher */}
          <button
            onClick={toggleLang}
            className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-slate-100 hover:bg-slate-200 text-slate-700 hover:text-slate-900 dark:bg-[#141C2E] dark:hover:bg-[#1C2740] dark:text-slate-300 dark:hover:text-white border border-transparent dark:border-slate-700/60 transition text-xs font-medium shadow-sm dark:shadow-none"
          >
            <Globe className="w-4 h-4 text-cyan-600 dark:text-cyan-400" />
            <span>{t.lang_toggle}</span>
          </button>
        </div>
      </div>

      {/* Second row: 4 Metric Cards + Live Graph */}
      <div className="grid grid-cols-1 md:grid-cols-4 lg:grid-cols-5 gap-3 mt-3.5 items-stretch">
        {/* Card 1: Live Download */}
        <div className="bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800/80 rounded-2xl p-3.5 flex items-center gap-3.5 shadow-sm dark:shadow-none transition-colors">
          <div className="p-2.5 rounded-xl bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 shrink-0">
            <ArrowDownCircle className="w-6 h-6" />
          </div>
          <div className="min-w-0 flex-1">
            <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 block mb-0.5">{t.live_download}</span>
            <span className="text-xl font-black text-emerald-600 dark:text-emerald-400 font-mono tracking-tight block">
              {formatSpeed(globalDownSpeed)}
            </span>
            <div className="text-[10px] text-slate-500 dark:text-slate-400 truncate mt-0.5">
              <span className="text-slate-400 dark:text-slate-500">{isRtl ? 'تنزيل اليوم' : "Today"}: </span>
              <span className="text-emerald-600 dark:text-emerald-400 font-semibold font-mono">{formatBytes(todayTotalDown)}</span>
            </div>
          </div>
        </div>

        {/* Card 2: Live Upload */}
        <div className="bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800/80 rounded-2xl p-3.5 flex items-center gap-3.5 shadow-sm dark:shadow-none transition-colors">
          <div className="p-2.5 rounded-xl bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 shrink-0">
            <ArrowUpCircle className="w-6 h-6" />
          </div>
          <div className="min-w-0 flex-1">
            <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 block mb-0.5">{t.live_upload}</span>
            <span className="text-xl font-black text-cyan-600 dark:text-cyan-400 font-mono tracking-tight block">
              {formatSpeed(globalUpSpeed)}
            </span>
            <div className="text-[10px] text-slate-500 dark:text-slate-400 truncate mt-0.5">
              <span className="text-slate-400 dark:text-slate-500">{isRtl ? 'رفع اليوم' : "Today"}: </span>
              <span className="text-cyan-600 dark:text-cyan-400 font-semibold font-mono">{formatBytes(todayTotalUp)}</span>
            </div>
          </div>
        </div>

        {/* Card 3: Active Sockets */}
        <div className="bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800/80 rounded-2xl p-3.5 flex items-center gap-3.5 shadow-sm dark:shadow-none transition-colors">
          <div className="p-2.5 rounded-xl bg-purple-500/10 text-purple-600 dark:text-purple-400 shrink-0">
            <Radio className="w-6 h-6" />
          </div>
          <div className="min-w-0 flex-1">
            <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 block mb-0.5">{t.active_connections}</span>
            <span className="text-xl font-black text-purple-600 dark:text-purple-400 font-mono tracking-tight block">
              {activeSocketsCount}
            </span>
            <div className="text-[10px] text-slate-400 dark:text-slate-500 truncate mt-0.5">
              {t.tracked_apps}: <span className="font-semibold text-purple-600 dark:text-purple-400 font-mono">{processes.length}</span>
            </div>
          </div>
        </div>

        {/* Card 4: WinDivert Processed Packets & Today's Total Usage */}
        <div className="bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800/80 rounded-2xl p-3.5 flex items-center gap-3.5 shadow-sm dark:shadow-none transition-colors">
          <div className="p-2.5 rounded-xl bg-amber-500/10 text-amber-600 dark:text-amber-400 shrink-0">
            <Database className="w-6 h-6" />
          </div>
          <div className="min-w-0 flex-1">
            <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 block mb-0.5">
              {isRtl ? 'استهلاك اليوم (إجمالي)' : "Today's Consumption"}
            </span>
            <span className="text-xl font-black text-amber-600 dark:text-amber-400 font-mono tracking-tight block">
              {formatBytes(todayTotalBytes)}
            </span>
            <div className="text-[10px] text-slate-400 dark:text-slate-500 truncate mt-0.5">
              {t.status_packets}: <span className="font-semibold text-amber-600 dark:text-amber-400 font-mono">{driverStatus.packet_count.toLocaleString()}</span>
            </div>
          </div>
        </div>

        {/* Card 5: Live Mini Sparkline */}
        <div className="md:col-span-4 lg:col-span-1 bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800/80 rounded-2xl p-2.5 shadow-sm dark:shadow-none flex flex-col justify-center">
          <BandwidthSparkline currentDown={globalDownSpeed} currentUp={globalUpSpeed} />
        </div>
      </div>
    </header>
  );
};
