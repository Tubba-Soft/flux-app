import React from 'react';
import { ProcessTraffic } from '../types';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { formatBytes } from '../utils/formatters';
import { FolderOpen, Skull, X, Terminal, Cpu, HardDrive } from 'lucide-react';

interface Props {
  process: ProcessTraffic | null;
  onClose: () => void;
}

export const ProcessDetailModal: React.FC<Props> = ({ process, onClose }) => {
  const { t } = useLanguage();
  const { openFileLocation, killProcess } = useTelemetry();

  if (!process) return null;

  const handleOpenExplorer = async () => {
    await openFileLocation(process.path);
  };

  const handleKill = async () => {
    if (window.confirm(`${t.action_kill_confirm}\n${process.name} (PID ${process.pid})`)) {
      await killProcess(process.pid);
      onClose();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/75 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      <div className="bg-white dark:bg-dark-surface border border-transparent dark:border-dark-border rounded-2xl max-w-lg w-full p-6 shadow-2xl relative text-slate-900 dark:text-slate-100">
        {/* Close Button */}
        <button
          onClick={onClose}
          className="absolute top-4 right-4 rtl:right-auto rtl:left-4 p-1.5 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-dark-hover transition"
        >
          <X className="w-5 h-5" />
        </button>

        {/* Process Header */}
        <div className="flex items-center gap-4 mb-5">
          {process.icon_base64 ? (
            <img src={process.icon_base64} alt={process.name} className="w-12 h-12 rounded-xl object-contain shadow-md" />
          ) : (
            <div className="w-12 h-12 rounded-xl bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border flex items-center justify-center text-slate-500 dark:text-slate-400 font-bold">
              <Cpu className="w-6 h-6 text-cyan-600 dark:text-brand-cyan" />
            </div>
          )}
          <div>
            <h3 className="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
              {process.name}
              <span className="text-xs px-2 py-0.5 rounded bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border text-slate-600 dark:text-slate-400 font-mono">
                PID: {process.pid}
              </span>
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 font-mono break-all mt-0.5">
              {process.path}
            </p>
          </div>
        </div>

        {/* Details Grid */}
        <div className="grid grid-cols-2 gap-3 mb-6">
          <div className="bg-slate-50 dark:bg-dark-bg p-3 rounded-xl border border-transparent dark:border-dark-border shadow-inner dark:shadow-none">
            <span className="text-[11px] text-slate-500 dark:text-slate-400 block mb-1">{t.active_connections}</span>
            <span className="text-sm font-bold text-slate-800 dark:text-slate-200 font-mono">
              {process.active_streams_count} Sockets
            </span>
          </div>
          <div className="bg-slate-50 dark:bg-dark-bg p-3 rounded-xl border border-transparent dark:border-dark-border shadow-inner dark:shadow-none">
            <span className="text-[11px] text-slate-500 dark:text-slate-400 block mb-1">{t.col_total_transfer}</span>
            <span className="text-sm font-bold text-slate-800 dark:text-slate-200 font-mono">
              {formatBytes(process.total_down_bytes + process.total_up_bytes)}
            </span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center justify-between gap-3 pt-4 border-t border-slate-200 dark:border-dark-border/40">
          <button
            onClick={handleKill}
            className="flex items-center gap-2 px-4 py-2 rounded-xl bg-rose-500/15 hover:bg-rose-500/25 text-rose-700 dark:text-rose-300 border border-transparent dark:border-rose-500/30 text-xs font-semibold transition shadow-sm dark:shadow-none"
          >
            <Skull className="w-4 h-4 text-rose-600 dark:text-rose-400" />
            <span>{t.action_kill_proc}</span>
          </button>

          <div className="flex items-center gap-2">
            <button
              onClick={handleOpenExplorer}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border hover:bg-slate-200 dark:hover:bg-dark-hover text-xs font-semibold text-slate-700 dark:text-slate-200 transition shadow-sm dark:shadow-none"
            >
              <FolderOpen className="w-4 h-4 text-cyan-600 dark:text-brand-cyan" />
              <span>{t.action_open_explorer}</span>
            </button>
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-slate-200 hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 text-xs font-semibold text-slate-800 dark:text-white transition"
            >
              {t.rule_cancel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
