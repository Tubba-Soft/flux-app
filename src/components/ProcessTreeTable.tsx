import React, { useState, useMemo } from 'react';
import { ProcessTraffic, StreamTraffic, SortField, SortDirection } from '../types';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { formatSpeed, formatBytes } from '../utils/formatters';
import { StreamRow } from './StreamRow';
import {
  ChevronRight,
  ChevronDown,
  Search,
  Sliders,
  Shield,
  ShieldOff,
  MoreVertical,
  FolderOpen,
  Info,
  Skull,
  ArrowUpDown,
  ArrowDown,
  ArrowUp,
  Cpu
} from 'lucide-react';

interface Props {
  onOpenRuleModal: (target: ProcessTraffic | StreamTraffic) => void;
  onOpenInspectModal: (proc: ProcessTraffic) => void;
  searchQuery?: string;
  onSearchQueryChange?: (q: string) => void;
}

export const ProcessTreeTable: React.FC<Props> = ({
  onOpenRuleModal,
  onOpenInspectModal,
  searchQuery: externalSearchQuery,
  onSearchQueryChange,
}) => {
  const { t, isRtl } = useLanguage();
  const { processes, updateProcessRule, openFileLocation, killProcess } = useTelemetry();

  // State: expanded process rows
  const [expandedPids, setExpandedPids] = useState<Set<number>>(new Set());
  // State: search query
  const [internalSearchQuery, setInternalSearchQuery] = useState<string>('');
  const searchQuery = externalSearchQuery !== undefined ? externalSearchQuery : internalSearchQuery;
  const setSearchQuery = (val: string) => {
    setInternalSearchQuery(val);
    onSearchQueryChange?.(val);
  };
  // State: sorting
  const [sortField, setSortField] = useState<SortField>('down_speed');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  // State: active menu dropdown
  const [activeMenuPid, setActiveMenuPid] = useState<number | null>(null);

  const toggleExpand = (pid: number) => {
    setExpandedPids((prev) => {
      const next = new Set(prev);
      if (next.has(pid)) next.delete(pid);
      else next.add(pid);
      return next;
    });
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  // Filter & Sort processes
  const filteredAndSortedProcesses = useMemo(() => {
    let result = [...processes];

    // Filter
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      result = result.filter((p) => {
        if (p.name.toLowerCase().includes(q)) return true;
        if (p.pid.toString().includes(q)) return true;
        if (p.path.toLowerCase().includes(q)) return true;
        return p.streams.some(
          (s) =>
            s.remote_ip.toLowerCase().includes(q) ||
            s.remote_port.toString().includes(q) ||
            s.local_port.toString().includes(q) ||
            (s.remote_domain && s.remote_domain.toLowerCase().includes(q))
        );
      });
    }

    // Sort
    result.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'down_speed':
          cmp = a.down_speed_bps - b.down_speed_bps;
          break;
        case 'up_speed':
          cmp = a.up_speed_bps - b.up_speed_bps;
          break;
        case 'total_bytes':
          cmp =
            a.total_down_bytes + a.total_up_bytes - (b.total_down_bytes + b.total_up_bytes);
          break;
        case 'name':
          cmp = a.name.localeCompare(b.name);
          break;
        case 'is_blocked':
          cmp = (a.is_blocked ? 1 : 0) - (b.is_blocked ? 1 : 0);
          break;
        case 'is_throttled':
          const aThrottled = a.down_limit_kbps || a.up_limit_kbps ? 1 : 0;
          const bThrottled = b.down_limit_kbps || b.up_limit_kbps ? 1 : 0;
          cmp = aThrottled - bThrottled;
          break;
        default:
          cmp = 0;
      }
      return sortDirection === 'desc' ? -cmp : cmp;
    });

    return result;
  }, [processes, searchQuery, sortField, sortDirection]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden bg-slate-100/60 dark:bg-[#090D16]">
      {/* Search & Sort Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 bg-white/70 dark:bg-[#0C1220]/80 border-b border-transparent dark:border-slate-800/60 backdrop-blur-sm">
        {/* Search */}
        <div className="relative flex-1 min-w-[280px] max-w-md">
          <Search className="absolute top-2.5 left-3 rtl:left-auto rtl:right-3 w-4 h-4 text-slate-400 dark:text-slate-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.search_placeholder}
            className="w-full bg-slate-100 dark:bg-[#121A2D] border border-transparent dark:border-slate-700/60 rounded-xl pl-9 pr-4 rtl:pr-9 rtl:pl-4 py-2 text-xs text-slate-900 dark:text-white placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500/40 transition shadow-inner dark:shadow-none"
          />
        </div>

        {/* Sort Controls */}
        <div className="flex items-center gap-2 text-xs">
          <span className="text-slate-500 dark:text-slate-400 font-medium">{t.sort_by}</span>
          <div className="flex items-center gap-1 bg-slate-200/80 dark:bg-[#121A2D] border border-transparent dark:border-slate-700/60 rounded-xl p-1 shadow-sm dark:shadow-none">
            {[
              { id: 'down_speed' as SortField, label: t.sort_down_speed },
              { id: 'up_speed' as SortField, label: t.sort_up_speed },
              { id: 'total_bytes' as SortField, label: t.sort_total_bytes },
              { id: 'name' as SortField, label: t.sort_name },
              { id: 'is_blocked' as SortField, label: t.sort_blocked },
              { id: 'is_throttled' as SortField, label: t.sort_throttled },
            ].map((s) => (
              <button
                key={s.id}
                onClick={() => handleSort(s.id)}
                className={`px-2.5 py-1 rounded-lg text-xs font-medium transition ${
                  sortField === s.id
                    ? 'bg-cyan-500/15 text-cyan-700 dark:text-cyan-400 border border-transparent dark:border-cyan-500/30 font-semibold shadow-sm'
                    : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-200 dark:hover:bg-slate-800/80'
                }`}
              >
                {s.label}
                {sortField === s.id && (
                  <span className="ml-1 rtl:mr-1 text-[10px]">
                    {sortDirection === 'desc' ? '↓' : '↑'}
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Table Container with Horizontal Scroll support */}
      <div className="flex-1 flex flex-col overflow-x-auto overflow-y-hidden">
        <div className="min-w-[980px] flex flex-col flex-1">
          {/* Table Header */}
          <div className="grid grid-cols-[minmax(260px,1.4fr)_130px_110px_110px_110px_140px_50px_40px] items-center px-6 py-2.5 bg-slate-200/60 dark:bg-[#0C1220] border-b border-transparent dark:border-slate-800/80 text-xs font-semibold text-slate-700 dark:text-slate-400 select-none shadow-sm dark:shadow-none">
            <div className="flex items-center gap-2">
              <span>{t.col_application}</span>
            </div>

            <div className="text-center">
              <span>{t.active_connections}</span>
            </div>

            <div
              className="flex items-center justify-center gap-1 cursor-pointer hover:text-slate-900 dark:hover:text-white transition"
              onClick={() => handleSort('down_speed')}
            >
              <ArrowDown className="w-3 h-3 text-emerald-600 dark:text-emerald-400" />
              <span>{t.col_down_speed}</span>
            </div>

            <div
              className="flex items-center justify-center gap-1 cursor-pointer hover:text-slate-900 dark:hover:text-white transition"
              onClick={() => handleSort('up_speed')}
            >
              <ArrowUp className="w-3 h-3 text-cyan-600 dark:text-cyan-400" />
              <span>{t.col_up_speed}</span>
            </div>

            <div
              className="text-center cursor-pointer hover:text-slate-900 dark:hover:text-white transition"
              onClick={() => handleSort('total_bytes')}
            >
              <span>{t.col_total_transfer}</span>
            </div>

            <div className="text-center">
              <span>{t.col_limits}</span>
            </div>

            <div className="text-center">
              <span>{t.col_block}</span>
            </div>

            <div className="text-center">
              <span>{t.col_actions}</span>
            </div>
          </div>

          {/* Table Body (Scrollable) */}
          <div className="flex-1 overflow-y-auto divide-y divide-slate-200/50 dark:divide-slate-800/30">
            {filteredAndSortedProcesses.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64 text-slate-500">
                <Cpu className="w-12 h-12 text-slate-400 dark:text-slate-600 mb-3" />
                <p className="text-sm">{isRtl ? 'لا توجد تطبيقات مطابقة لنتائج البحث' : 'No active applications matching criteria'}</p>
              </div>
            ) : (
              filteredAndSortedProcesses.map((proc) => {
                const isExpanded = expandedPids.has(proc.pid);
                const hasLimits = proc.down_limit_kbps || proc.up_limit_kbps;

                return (
                  <div key={proc.pid} className="group">
                    {/* Process Row */}
                    <div
                      onClick={() => toggleExpand(proc.pid)}
                      className={`grid grid-cols-[minmax(260px,1.4fr)_130px_110px_110px_110px_140px_50px_40px] items-center px-6 py-2.5 cursor-pointer border-b border-transparent dark:border-slate-800/20 transition-colors duration-150 ${
                        proc.is_blocked
                          ? 'bg-rose-500/10 dark:bg-rose-950/20 hover:bg-rose-500/15 dark:hover:bg-rose-950/30'
                          : isExpanded
                          ? 'bg-slate-100/90 dark:bg-[#11192A]'
                          : 'bg-white even:bg-slate-50/70 hover:bg-slate-100/90 dark:bg-[#090D16] dark:even:bg-[#0C1220] dark:hover:bg-[#162238]'
                      }`}
                    >
                      {/* 1. App Info (Chevron + Icon + Name + PID + Path) */}
                      <div className="flex items-center gap-2.5 min-w-0 pr-2 rtl:pr-0 rtl:pl-2">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleExpand(proc.pid);
                          }}
                          className="p-1 rounded text-slate-400 hover:text-slate-700 dark:text-slate-500 dark:hover:text-slate-200 shrink-0 transition"
                        >
                          {isExpanded ? (
                            <ChevronDown className="w-4 h-4 text-cyan-500" />
                          ) : (
                            <ChevronRight className={`w-4 h-4 ${isRtl ? 'rotate-180' : ''}`} />
                          )}
                        </button>

                        <div className="w-8 h-8 rounded-lg bg-slate-100 dark:bg-[#141C2E] border border-transparent dark:border-slate-700/60 flex items-center justify-center overflow-hidden shrink-0 shadow-sm dark:shadow-none">
                          {proc.icon_base64 ? (
                            <img src={proc.icon_base64} alt={proc.name} className="w-6 h-6 object-contain" />
                          ) : (
                            <Cpu className="w-4 h-4 text-slate-500 dark:text-slate-400" />
                          )}
                        </div>

                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-1.5 flex-wrap">
                            <span className="text-xs font-bold text-slate-900 dark:text-slate-100 truncate group-hover:text-cyan-600 dark:group-hover:text-cyan-400 transition" title={proc.name}>
                              {proc.name}
                            </span>
                            <span className="text-[10px] px-1.5 py-0.2 rounded bg-slate-200/80 dark:bg-slate-800 border border-transparent dark:border-slate-700/60 text-slate-600 dark:text-slate-400 font-mono shrink-0">
                              {proc.pid}
                            </span>
                            {proc.priority !== 'Normal' && (
                              <span className="text-[9px] px-1 rounded bg-purple-100 dark:bg-purple-950/60 border border-transparent dark:border-purple-700/40 text-purple-700 dark:text-purple-300 font-semibold shrink-0">
                                {proc.priority}
                              </span>
                            )}
                          </div>
                          <span className="text-[10px] text-slate-400 dark:text-slate-500 font-mono truncate block" title={proc.path}>
                            {proc.path}
                          </span>
                        </div>
                      </div>

                      {/* 2. Active Sockets Badge */}
                      <div className="flex justify-center">
                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold bg-slate-200/70 dark:bg-slate-800/80 border border-transparent dark:border-slate-700/60 text-slate-700 dark:text-slate-300 shadow-sm dark:shadow-none">
                          {proc.active_streams_count} {t.active_connections}
                        </span>
                      </div>

                      {/* 3. Down Speed */}
                      <div className="flex flex-col items-center justify-center px-1">
                        <span className={`font-mono text-xs font-bold ${proc.down_speed_bps > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-slate-400 dark:text-slate-600'}`}>
                          {formatSpeed(proc.down_speed_bps)}
                        </span>
                        <div className="w-16 h-1 bg-slate-200 dark:bg-slate-800 rounded-full mt-1 overflow-hidden">
                          <div
                            className="h-full bg-emerald-500 dark:bg-emerald-400 rounded-full transition-all duration-300"
                            style={{ width: `${Math.min(100, (proc.down_speed_bps / (1024 * 1024 * 10)) * 100)}%` }}
                          />
                        </div>
                      </div>

                      {/* 4. Up Speed */}
                      <div className="flex flex-col items-center justify-center px-1">
                        <span className={`font-mono text-xs font-bold ${proc.up_speed_bps > 0 ? 'text-cyan-600 dark:text-cyan-400' : 'text-slate-400 dark:text-slate-600'}`}>
                          {formatSpeed(proc.up_speed_bps)}
                        </span>
                        <div className="w-16 h-1 bg-slate-200 dark:bg-slate-800 rounded-full mt-1 overflow-hidden">
                          <div
                            className="h-full bg-cyan-500 dark:bg-cyan-400 rounded-full transition-all duration-300"
                            style={{ width: `${Math.min(100, (proc.up_speed_bps / (1024 * 1024 * 2)) * 100)}%` }}
                          />
                        </div>
                      </div>

                      {/* 5. Total Data */}
                      <div className="text-center font-mono text-xs text-slate-700 dark:text-slate-300">
                        {formatBytes(proc.total_down_bytes + proc.total_up_bytes)}
                      </div>

                      {/* 6. Limiter Button */}
                      <div className="flex justify-center" onClick={(e) => e.stopPropagation()}>
                        <button
                          onClick={() => onOpenRuleModal(proc)}
                          className={`w-full max-w-[130px] px-2.5 py-1 rounded-lg text-xs font-medium flex items-center justify-center gap-1.5 transition truncate ${
                            hasLimits
                              ? 'bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-transparent dark:border-amber-500/40 font-bold'
                              : 'bg-slate-200/80 dark:bg-slate-800/80 hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 border border-transparent dark:border-slate-700/60 shadow-sm dark:shadow-none'
                          }`}
                          title={t.action_shape_traffic}
                        >
                          <Sliders className="w-3.5 h-3.5 shrink-0" />
                          <span className="truncate">{proc.down_limit_kbps ? `${proc.down_limit_kbps} KB/s` : t.rule_no_limit}</span>
                        </button>
                      </div>

                      {/* 7. Block Toggle Switch */}
                      <div className="flex justify-center" onClick={(e) => e.stopPropagation()}>
                        <button
                          onClick={() =>
                            updateProcessRule({
                              path: proc.path,
                              name: proc.name,
                              is_blocked: !proc.is_blocked,
                              down_limit_kbps: proc.down_limit_kbps,
                              up_limit_kbps: proc.up_limit_kbps,
                              priority: proc.priority,
                            })
                          }
                          className={`p-1.5 rounded-lg border transition ${
                            proc.is_blocked
                              ? 'bg-rose-500/15 border-transparent dark:border-rose-500/50 text-rose-600 dark:text-rose-400'
                              : 'bg-slate-200/80 dark:bg-slate-800/80 border-transparent dark:border-slate-700/60 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-300 dark:hover:bg-slate-700 shadow-sm dark:shadow-none'
                          }`}
                          title={t.col_block}
                        >
                          {proc.is_blocked ? (
                            <ShieldOff className="w-4 h-4 text-rose-600 dark:text-rose-400" />
                          ) : (
                            <Shield className="w-4 h-4" />
                          )}
                        </button>
                      </div>

                      {/* 8. Action Dropdown Menu */}
                      <div className="flex justify-center relative" onClick={(e) => e.stopPropagation()}>
                        <button
                          onClick={() =>
                            setActiveMenuPid(activeMenuPid === proc.pid ? null : proc.pid)
                          }
                          className="p-1.5 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white transition"
                        >
                          <MoreVertical className="w-4 h-4" />
                        </button>

                        {activeMenuPid === proc.pid && (
                          <div className="absolute right-0 rtl:right-auto rtl:left-0 top-full mt-1 w-48 bg-white dark:bg-[#121A2D] border border-transparent dark:border-slate-800 rounded-xl shadow-2xl z-40 py-1 text-xs">
                            <button
                              onClick={() => {
                                openFileLocation(proc.path);
                                setActiveMenuPid(null);
                              }}
                              className="w-full flex items-center gap-2 px-3 py-2 text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800/80 hover:text-slate-900 dark:hover:text-white text-left rtl:text-right transition"
                            >
                              <FolderOpen className="w-3.5 h-3.5 text-cyan-600 dark:text-cyan-400" />
                              <span>{t.action_open_explorer}</span>
                            </button>

                            <button
                              onClick={() => {
                                onOpenInspectModal(proc);
                                setActiveMenuPid(null);
                              }}
                              className="w-full flex items-center gap-2 px-3 py-2 text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800/80 hover:text-slate-900 dark:hover:text-white text-left rtl:text-right transition"
                            >
                              <Info className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
                              <span>{t.action_inspect}</span>
                            </button>

                            <button
                              onClick={() => {
                                onOpenRuleModal(proc);
                                setActiveMenuPid(null);
                              }}
                              className="w-full flex items-center gap-2 px-3 py-2 text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800/80 hover:text-slate-900 dark:hover:text-white text-left rtl:text-right transition"
                            >
                              <Sliders className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />
                              <span>{t.action_shape_traffic}</span>
                            </button>

                            <div className="my-1 border-t border-slate-200 dark:border-slate-800/60" />

                            <button
                              onClick={() => {
                                if (window.confirm(`${t.action_kill_confirm}\n${proc.name}`)) {
                                  killProcess(proc.pid);
                                }
                                setActiveMenuPid(null);
                              }}
                              className="w-full flex items-center gap-2 px-3 py-2 text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 text-left rtl:text-right transition"
                            >
                              <Skull className="w-3.5 h-3.5" />
                              <span>{t.action_kill_proc}</span>
                            </button>
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Sub-Rows: Individual Connection Streams */}
                    {isExpanded && (
                      <div className="bg-slate-100/50 dark:bg-[#070A12] border-y border-transparent dark:border-slate-800/40">
                        {proc.streams.length === 0 ? (
                          <div className="py-2.5 px-6 text-slate-400 dark:text-slate-500 text-xs italic">
                            {isRtl ? 'لا توجد مسارات شبكة نشطة مفتوحة لهذا التطبيق حالياً' : 'No active network sockets open for this process'}
                          </div>
                        ) : (
                          proc.streams.map((stream) => (
                            <StreamRow
                              key={stream.id}
                              stream={stream}
                              onOpenRuleModal={onOpenRuleModal}
                            />
                          ))
                        )}
                      </div>
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
