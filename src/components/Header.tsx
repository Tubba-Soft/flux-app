import React, { useState } from 'react';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { formatSpeed, formatBytes } from '../utils/formatters';
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
    <header className="bg-dark-surface border-b border-dark-border/60 p-4 shadow-lg backdrop-blur-md">
      {/* Top row: Brand + Kernel Status + Language Switcher */}
      <div className="flex flex-wrap items-center justify-between gap-4 pb-3 border-b border-dark-border/40">
        {/* Brand */}
        <div className="flex items-center gap-3">
          <div className="relative flex items-center justify-center w-10 h-10 rounded-xl bg-gradient-to-tr from-brand-emerald via-brand-cyan to-brand-purple p-[1.5px] shadow-lg shadow-brand-cyan/20">
            <div className="w-full h-full bg-dark-bg rounded-[10px] flex items-center justify-center">
              <Activity className="w-5 h-5 text-brand-cyan animate-pulse" />
            </div>
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent">
              {t.app_name}
            </h1>
            <p className="text-xs text-slate-400 font-medium">
              {t.tagline}
            </p>
          </div>
        </div>

        {/* Driver Status Pill & Actions */}
        <div className="flex items-center flex-wrap gap-2">
          {/* Driver Status Badge */}
          <div
            className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-semibold border ${
              isKernelActive
                ? 'bg-emerald-950/40 text-emerald-400 border-emerald-500/30'
                : 'bg-amber-950/40 text-amber-400 border-amber-500/30'
            }`}
          >
            {isKernelActive ? (
              <>
                <ShieldCheck className="w-4 h-4 text-emerald-400" />
                <span>{t.status_kernel_active}</span>
                <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
              </>
            ) : (
              <>
                <ShieldAlert className="w-4 h-4 text-amber-400" />
                <span>{t.status_passive_monitor}</span>
                <button
                  onClick={elevateAdmin}
                  className="ml-2 px-2 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 font-bold underline transition"
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
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium transition ${
              taskbarWidget
                ? 'bg-emerald-500/15 border-emerald-500/40 text-emerald-300 shadow-sm shadow-emerald-500/20'
                : 'bg-dark-card border-dark-border text-slate-400 hover:text-slate-200 hover:bg-dark-hover'
            }`}
            title={isRtl ? 'عرض ودجت شريط المهام في أسفل الشاشة بجانب الساعة والواي فاي والبطارية' : 'Show Taskbar Speed Widget near clock and battery'}
          >
            <Gauge className="w-3.5 h-3.5" />
            <span>{isRtl ? 'ودجت المهام' : 'Taskbar Widget'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${taskbarWidget ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'}`} />
          </button>

          {/* Widget Customization / Settings Trigger */}
          <button
            onClick={() => onOpenWidgetSettings?.()}
            className="p-2 rounded-lg bg-dark-card border border-dark-border text-slate-400 hover:text-brand-cyan hover:border-brand-cyan/40 hover:bg-dark-hover transition"
            title={isRtl ? 'تخصيص ودجت شريط المهام (تثبيت، شفافية، ألوان)' : 'Widget Customization (Docking, Transparency, Themes)'}
          >
            <Sliders className="w-3.5 h-3.5" />
          </button>

          {/* Background Running Toggle */}
          <button
            onClick={() => setRunInBackground(!runInBackground)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium transition ${
              runInBackground
                ? 'bg-cyan-500/15 border-cyan-500/40 text-cyan-300 shadow-sm shadow-cyan-500/20'
                : 'bg-dark-card border-dark-border text-slate-400 hover:text-slate-200 hover:bg-dark-hover'
            }`}
            title={isRtl ? 'التشغيل في الخلفية بصينية النظام عند إغلاق النافذة' : 'Run in background (System Tray) when closed'}
          >
            <Minimize2 className="w-3.5 h-3.5" />
            <span>{isRtl ? 'بالخلفية' : 'Tray Mode'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${runInBackground ? 'bg-cyan-400' : 'bg-slate-500'}`} />
          </button>

          {/* Windows Autostart Toggle */}
          <button
            onClick={() => setAutostart(!autostart)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium transition ${
              autostart
                ? 'bg-purple-500/15 border-purple-500/40 text-purple-300 shadow-sm shadow-purple-500/20'
                : 'bg-dark-card border-dark-border text-slate-400 hover:text-slate-200 hover:bg-dark-hover'
            }`}
            title={isRtl ? 'التشغيل التلقائي مع إقلاع نظام ويندوز' : 'Start automatically on Windows boot'}
          >
            <Power className="w-3.5 h-3.5" />
            <span>{isRtl ? 'بدء تلقائي' : 'Autostart'}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${autostart ? 'bg-purple-400' : 'bg-slate-500'}`} />
          </button>

          {/* Info / About Button */}
          <button
            onClick={() => onOpenAbout?.()}
            className="p-2 rounded-lg bg-dark-card border border-dark-border text-slate-400 hover:text-slate-200 hover:bg-dark-hover transition"
            title={t.av_notice_title}
          >
            <Info className="w-4 h-4" />
          </button>

          {/* Language Switcher */}
          <button
            onClick={toggleLang}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-dark-card border border-dark-border hover:border-brand-cyan/40 hover:bg-dark-hover transition text-xs font-medium text-slate-300"
          >
            <Globe className="w-4 h-4 text-brand-cyan" />
            <span>{t.lang_toggle}</span>
          </button>
        </div>
      </div>

      {/* Second row: 4 Metric Cards + Live Graph */}
      <div className="grid grid-cols-1 md:grid-cols-4 lg:grid-cols-5 gap-3 mt-3 items-center">
        {/* Card 1: Live Download */}
        <div className="bg-dark-card/70 border border-dark-border/50 rounded-xl p-3 flex items-center gap-3">
          <div className="p-2 rounded-lg bg-emerald-500/10 text-brand-emerald">
            <ArrowDownCircle className="w-6 h-6" />
          </div>
          <div>
            <span className="text-[11px] font-medium text-slate-400 block">{t.live_download}</span>
            <span className="text-lg font-bold text-brand-emerald font-mono">
              {formatSpeed(globalDownSpeed)}
            </span>
            <div className="text-[10px] text-slate-400">
              <span className="text-slate-500">{isRtl ? 'تنزيل اليوم' : "Today"}: </span>
              <span className="text-emerald-400 font-semibold font-mono">{formatBytes(todayTotalDown)}</span>
            </div>
          </div>
        </div>

        {/* Card 2: Live Upload */}
        <div className="bg-dark-card/70 border border-dark-border/50 rounded-xl p-3 flex items-center gap-3">
          <div className="p-2 rounded-lg bg-cyan-500/10 text-brand-cyan">
            <ArrowUpCircle className="w-6 h-6" />
          </div>
          <div>
            <span className="text-[11px] font-medium text-slate-400 block">{t.live_upload}</span>
            <span className="text-lg font-bold text-brand-cyan font-mono">
              {formatSpeed(globalUpSpeed)}
            </span>
            <div className="text-[10px] text-slate-400">
              <span className="text-slate-500">{isRtl ? 'رفع اليوم' : "Today"}: </span>
              <span className="text-cyan-400 font-semibold font-mono">{formatBytes(todayTotalUp)}</span>
            </div>
          </div>
        </div>

        {/* Card 3: Active Sockets */}
        <div className="bg-dark-card/70 border border-dark-border/50 rounded-xl p-3 flex items-center gap-3">
          <div className="p-2 rounded-lg bg-purple-500/10 text-brand-purple">
            <Radio className="w-6 h-6" />
          </div>
          <div>
            <span className="text-[11px] font-medium text-slate-400 block">{t.active_connections}</span>
            <span className="text-lg font-bold text-purple-400 font-mono">
              {activeSocketsCount}
            </span>
            <div className="text-[10px] text-slate-500">
              {t.tracked_apps}: {processes.length}
            </div>
          </div>
        </div>

        {/* Card 4: WinDivert Processed Packets & Today's Total Usage */}
        <div className="bg-dark-card/70 border border-dark-border/50 rounded-xl p-3 flex items-center gap-3">
          <div className="p-2 rounded-lg bg-amber-500/10 text-brand-amber">
            <Database className="w-6 h-6" />
          </div>
          <div>
            <span className="text-[11px] font-medium text-slate-400 block">
              {isRtl ? 'استهلاك اليوم (إجمالي)' : "Today's Consumption"}
            </span>
            <span className="text-lg font-bold text-amber-400 font-mono">
              {formatBytes(todayTotalBytes)}
            </span>
            <div className="text-[10px] text-slate-500">
              {t.status_packets}: {driverStatus.packet_count.toLocaleString()}
            </div>
          </div>
        </div>

        {/* Card 5: Live Mini Sparkline */}
        <div className="md:col-span-4 lg:col-span-1">
          <BandwidthSparkline currentDown={globalDownSpeed} currentUp={globalUpSpeed} />
        </div>
      </div>
    </header>
  );
};
