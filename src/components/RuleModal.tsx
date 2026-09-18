import React, { useState } from 'react';
import { AppRule, ProcessTraffic, StreamRule, StreamTraffic } from '../types';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { Sliders, ShieldOff, Zap, X } from 'lucide-react';

interface Props {
  target: ProcessTraffic | StreamTraffic | null;
  onClose: () => void;
}

export const RuleModal: React.FC<Props> = ({ target, onClose }) => {
  const { t } = useLanguage();
  const { updateProcessRule, updateStreamRule } = useTelemetry();

  if (!target) return null;

  const isProcess = 'pid' in target;

  const [downLimit, setDownLimit] = useState<string>(
    target.down_limit_kbps ? target.down_limit_kbps.toString() : ''
  );
  const [upLimit, setUpLimit] = useState<string>(
    target.up_limit_kbps ? target.up_limit_kbps.toString() : ''
  );
  const [isBlocked, setIsBlocked] = useState<boolean>(target.is_blocked || false);
  const [priority, setPriority] = useState<string>(
    isProcess ? (target as ProcessTraffic).priority || 'Normal' : 'Normal'
  );

  const handleSave = async () => {
    const parsedDown = downLimit ? parseInt(downLimit, 10) : null;
    const parsedUp = upLimit ? parseInt(upLimit, 10) : null;

    if (isProcess) {
      const proc = target as ProcessTraffic;
      await updateProcessRule({
        path: proc.path,
        name: proc.name,
        is_blocked: isBlocked,
        down_limit_kbps: parsedDown && parsedDown > 0 ? parsedDown : null,
        up_limit_kbps: parsedUp && parsedUp > 0 ? parsedUp : null,
        priority,
      });
    } else {
      const st = target as StreamTraffic;
      await updateStreamRule({
        stream_id: st.id,
        is_blocked: isBlocked,
        down_limit_kbps: parsedDown && parsedDown > 0 ? parsedDown : null,
        up_limit_kbps: parsedUp && parsedUp > 0 ? parsedUp : null,
      });
    }
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/75 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      <div className="bg-white dark:bg-dark-surface border border-transparent dark:border-dark-border rounded-2xl max-w-md w-full p-6 shadow-2xl relative text-slate-900 dark:text-slate-100">
        {/* Close Button */}
        <button
          onClick={onClose}
          className="absolute top-4 right-4 rtl:right-auto rtl:left-4 p-1.5 rounded-lg text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-dark-hover transition"
        >
          <X className="w-5 h-5" />
        </button>

        {/* Title */}
        <div className="flex items-center gap-3 mb-5">
          <div className="p-2.5 rounded-xl bg-cyan-500/10 dark:bg-brand-cyan/10 border border-transparent dark:border-brand-cyan/20 text-cyan-600 dark:text-brand-cyan">
            <Sliders className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-900 dark:text-white">
              {isProcess ? t.rule_title_proc : t.rule_title_stream}
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 truncate max-w-[280px]">
              {isProcess ? (target as ProcessTraffic).name : (target as StreamTraffic).id}
            </p>
          </div>
        </div>

        {/* Controls */}
        <div className="space-y-4">
          {/* Download Cap */}
          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
              {t.rule_down_limit} (KB/s)
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={downLimit}
                onChange={(e) => setDownLimit(e.target.value)}
                placeholder={t.rule_no_limit}
                className="flex-1 bg-slate-100 dark:bg-dark-bg border border-transparent dark:border-dark-border rounded-lg px-3 py-2 text-sm text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-emerald-500/50"
              />
              <button
                type="button"
                onClick={() => setDownLimit('')}
                className="px-2.5 py-2 rounded-lg bg-slate-200/80 dark:bg-dark-card border border-transparent dark:border-dark-border text-xs text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-300/80 dark:hover:bg-dark-hover"
              >
                {t.clear_filter}
              </button>
            </div>
            <div className="flex gap-1.5 mt-1.5">
              {[256, 1024, 5120, 10240].map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => setDownLimit(preset.toString())}
                  className="px-2 py-0.5 rounded bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border text-[10px] text-slate-600 dark:text-slate-400 hover:text-emerald-600 dark:hover:text-brand-emerald hover:bg-slate-200 dark:hover:bg-dark-hover"
                >
                  {preset >= 1024 ? `${preset / 1024} MB/s` : `${preset} KB/s`}
                </button>
              ))}
            </div>
          </div>

          {/* Upload Cap */}
          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
              {t.rule_up_limit} (KB/s)
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={upLimit}
                onChange={(e) => setUpLimit(e.target.value)}
                placeholder={t.rule_no_limit}
                className="flex-1 bg-slate-100 dark:bg-dark-bg border border-transparent dark:border-dark-border rounded-lg px-3 py-2 text-sm text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-cyan-500/50"
              />
              <button
                type="button"
                onClick={() => setUpLimit('')}
                className="px-2.5 py-2 rounded-lg bg-slate-200/80 dark:bg-dark-card border border-transparent dark:border-dark-border text-xs text-slate-700 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-300/80 dark:hover:bg-dark-hover"
              >
                {t.clear_filter}
              </button>
            </div>
            <div className="flex gap-1.5 mt-1.5">
              {[128, 512, 2048, 5120].map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => setUpLimit(preset.toString())}
                  className="px-2 py-0.5 rounded bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border text-[10px] text-slate-600 dark:text-slate-400 hover:text-cyan-600 dark:hover:text-brand-cyan hover:bg-slate-200 dark:hover:bg-dark-hover"
                >
                  {preset >= 1024 ? `${preset / 1024} MB/s` : `${preset} KB/s`}
                </button>
              ))}
            </div>
          </div>

          {/* Priority (Process only) */}
          {isProcess && (
            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                {t.rule_priority}
              </label>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { id: 'High', label: t.rule_priority_high, color: 'hover:border-purple-500' },
                  { id: 'Normal', label: t.rule_priority_normal, color: 'hover:border-blue-500' },
                  { id: 'Low', label: t.rule_priority_low, color: 'hover:border-slate-500' },
                ].map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    onClick={() => setPriority(p.id)}
                    className={`py-2 px-2 text-xs font-medium rounded-lg border text-center transition ${
                      priority === p.id
                        ? 'bg-cyan-500/15 border-transparent dark:border-brand-cyan text-cyan-800 dark:text-brand-cyan font-bold shadow-sm'
                        : `bg-slate-100 dark:bg-dark-bg border-transparent dark:border-dark-border text-slate-700 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-dark-hover ${p.color}`
                    }`}
                  >
                    {p.id}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Block Toggle */}
          <div className="pt-2 border-t border-slate-200 dark:border-dark-border/40">
            <label className="flex items-center justify-between p-3 rounded-xl bg-slate-100 dark:bg-dark-bg border border-transparent dark:border-dark-border cursor-pointer hover:bg-slate-200/70 dark:hover:bg-dark-hover transition">
              <div className="flex items-center gap-2">
                <ShieldOff className={`w-4 h-4 ${isBlocked ? 'text-rose-600 dark:text-rose-400' : 'text-slate-400'}`} />
                <span className={`text-xs font-semibold ${isBlocked ? 'text-rose-600 dark:text-rose-400' : 'text-slate-700 dark:text-slate-300'}`}>
                  {t.rule_block_all}
                </span>
              </div>
              <input
                type="checkbox"
                checked={isBlocked}
                onChange={(e) => setIsBlocked(e.target.checked)}
                className="w-4 h-4 text-rose-500 rounded bg-white dark:bg-dark-surface border-transparent dark:border-dark-border focus:ring-rose-500"
              />
            </label>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center justify-end gap-3 mt-6">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 rounded-xl bg-slate-100 dark:bg-dark-card border border-transparent dark:border-dark-border text-xs font-semibold text-slate-700 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-dark-hover transition"
          >
            {t.rule_cancel}
          </button>
          <button
            type="button"
            onClick={handleSave}
            className="px-5 py-2 rounded-xl bg-gradient-to-r from-brand-emerald to-brand-cyan text-slate-950 font-bold text-xs hover:opacity-90 transition shadow-lg shadow-brand-cyan/20"
          >
            {t.rule_save}
          </button>
        </div>
      </div>
    </div>
  );
};
