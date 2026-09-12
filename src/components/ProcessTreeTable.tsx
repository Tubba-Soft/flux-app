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
    <div className="flex-1 flex flex-col overflow-hidden bg-dark-bg">
      {/* Search & Sort Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 bg-dark-surface/60 border-b border-dark-border/40 backdrop-blur-sm">
        {/* Search */}
        <div className="relative flex-1 min-w-[280px] max-w-md">
          <Search className="absolute top-2.5 left-3 rtl:left-auto rtl:right-3 w-4 h-4 text-slate-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.search_placeholder}
            className="w-full bg-dark-card/90 border border-dark-border rounded-xl pl-9 pr-4 rtl:pr-9 rtl:pl-4 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-brand-cyan transition"
          />
        </div>

        {/* Sort Controls */}
        <div className="flex items-center gap-2 text-xs">
          <span className="text-slate-400 font-medium">{t.sort_by}</span>
          <div className="flex items-center gap-1 bg-dark-card border border-dark-border rounded-xl p-1">
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
                    ? 'bg-brand-cyan/20 text-brand-cyan border border-brand-cyan/30'
                    : 'text-slate-400 hover:text-white hover:bg-dark-hover'
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

      {/* Table Header */}
      <div className="flex items-center justify-between px-6 py-2.5 bg-dark-surface/90 border-b border-dark-border/60 text-xs font-semibold text-slate-400 select-none">
        <div className="flex items-center gap-3 min-w-[320px] flex-1">
          <span>{t.col_application}</span>
        </div>

        <div className="flex items-center gap-8 min-w-[300px]">
          <div className="w-24 text-right flex items-center justify-end gap-1 cursor-pointer hover:text-white" onClick={() => handleSort('down_speed')}>
            <ArrowDown className="w-3 h-3 text-brand-emerald" />
            <span>{t.col_down_speed}</span>
          </div>

          <div className="w-24 text-right flex items-center justify-end gap-1 cursor-pointer hover:text-white" onClick={() => handleSort('up_speed')}>
            <ArrowUp className="w-3 h-3 text-brand-cyan" />
            <span>{t.col_up_speed}</span>
          </div>

          <div className="w-24 text-right cursor-pointer hover:text-white" onClick={() => handleSort('total_bytes')}>
            <span>{t.col_total_transfer}</span>
          </div>
        </div>

        <div className="flex items-center gap-6 min-w-[170px] justify-end">
          <span>{t.col_limits}</span>
          <span>{t.col_block}</span>
          <span>{t.col_actions}</span>
        </div>
      </div>

      {/* Table Body (Virtualized / Scrollable) */}
      <div className="flex-1 overflow-y-auto divide-y divide-dark-border/30">
        {filteredAndSortedProcesses.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-slate-500">
            <Cpu className="w-12 h-12 text-slate-600 mb-3" />
            <p className="text-sm">No active applications matching criteria</p>
          </div>
        ) : (
          filteredAndSortedProcesses.map((proc) => {
            const isExpanded = expandedPids.has(proc.pid);
            const hasStreams = proc.streams && proc.streams.length > 0;
            const hasLimits = proc.down_limit_kbps || proc.up_limit_kbps;

            return (
              <div key={proc.pid} className="group transition-colors">
                {/* Process Row */}
                <div
                  onClick={() => toggleExpand(proc.pid)}
                  className={`flex items-center justify-between py-2.5 px-6 cursor-pointer border-b border-dark-border/20 transition ${
                    isExpanded
                      ? 'bg-dark-card/60'
                      : 'hover:bg-dark-card/30'
                  } ${proc.is_blocked ? 'bg-rose-950/10' : ''}`}
                >
                  {/* Left: Expand + Icon + Name + PID + Streams Count */}
                  <div className="flex items-center gap-3 min-w-[320px] flex-1">
                    {/* Expand Chevron */}
                    <button
                      type="button"
                      className="p-1 rounded text-slate-400 hover:text-white"
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4 text-brand-cyan" />
                      ) : (
                        <ChevronRight className="w-4 h-4" />
                      )}
                    </button>

                    {/* App Icon */}
                    <div className="w-8 h-8 rounded-lg bg-dark-card border border-dark-border/80 flex items-center justify-center overflow-hidden shrink-0 shadow-sm">
                      {proc.icon_base64 ? (
                        <img src={proc.icon_base64} alt={proc.name} className="w-6 h-6 object-contain" />
                      ) : (
                        <Cpu className="w-4 h-4 text-slate-400" />
                      )}
                    </div>

                    {/* Process Info */}
                    <div className="truncate max-w-[340px]">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-white group-hover:text-brand-cyan transition">
                          {proc.name}
                        </span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-dark-card border border-dark-border/60 text-slate-400 font-mono">
                          {proc.pid}
                        </span>
                        {proc.priority !== 'Normal' && (
                          <span className="text-[9px] px-1 rounded bg-purple-900/40 border border-purple-700/40 text-purple-300 font-semibold">
                            {proc.priority}
                          </span>
                        )}
                      </div>
                      <span className="text-[10px] text-slate-500 font-mono truncate block" title={proc.path}>
                        {proc.path}
                      </span>
                    </div>

                    {/* Active Streams Badge */}
                    <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-dark-card border border-dark-border/60 text-slate-300">
                      {proc.active_streams_count} {t.active_connections}
                    </span>
                  </div>

                  {/* Middle: Speeds & Total Data */}
                  <div className="flex items-center gap-8 min-w-[300px]">
                    {/* Down speed */}
                    <div className="w-24 text-right">
                      <div className={`font-mono text-xs font-bold ${proc.down_speed_bps > 0 ? 'text-brand-emerald' : 'text-slate-500'}`}>
                        {formatSpeed(proc.down_speed_bps)}
                      </div>
                      {/* Mini visual indicator */}
                      <div className="w-full h-1 bg-dark-border/40 rounded-full mt-1 overflow-hidden">
                        <div
                          className="h-full bg-brand-emerald rounded-full transition-all duration-300"
                          style={{ width: `${Math.min(100, (proc.down_speed_bps / (1024 * 1024 * 10)) * 100)}%` }}
                        />
                      </div>
                    </div>

                    {/* Up speed */}
                    <div className="w-24 text-right">
                      <div className={`font-mono text-xs font-bold ${proc.up_speed_bps > 0 ? 'text-brand-cyan' : 'text-slate-500'}`}>
                        {formatSpeed(proc.up_speed_bps)}
                      </div>
                      <div className="w-full h-1 bg-dark-border/40 rounded-full mt-1 overflow-hidden">
                        <div
                          className="h-full bg-brand-cyan rounded-full transition-all duration-300"
                          style={{ width: `${Math.min(100, (proc.up_speed_bps / (1024 * 1024 * 2)) * 100)}%` }}
                        />
                      </div>
                    </div>

                    {/* Total Data */}
                    <div className="w-24 text-right font-mono text-xs text-slate-300">
                      {formatBytes(proc.total_down_bytes + proc.total_up_bytes)}
                    </div>
                  </div>

                  {/* Right: Controls & Actions */}
                  <div className="flex items-center gap-4 min-w-[170px] justify-end" onClick={(e) => e.stopPropagation()}>
                    {/* Limiter Button */}
                    <button
                      onClick={() => onOpenRuleModal(proc)}
                      className={`px-2.5 py-1 rounded-lg text-xs font-medium flex items-center gap-1.5 transition ${
                        hasLimits
                          ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40 font-bold'
                          : 'bg-dark-card hover:bg-dark-hover text-slate-400 border border-dark-border'
                      }`}
                      title={t.action_shape_traffic}
                    >
                      <Sliders className="w-3.5 h-3.5" />
                      <span>{proc.down_limit_kbps ? `${proc.down_limit_kbps} KB/s` : t.rule_no_limit}</span>
                    </button>

                    {/* Block Toggle Switch */}
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
                          ? 'bg-rose-500/20 border-rose-500/50 text-rose-400'
                          : 'bg-dark-card border-dark-border text-slate-500 hover:text-slate-300 hover:bg-dark-hover'
                      }`}
                      title={t.col_block}
                    >
                      {proc.is_blocked ? (
                        <ShieldOff className="w-4 h-4 text-rose-400" />
                      ) : (
                        <Shield className="w-4 h-4" />
                      )}
                    </button>

                    {/* Action Dropdown Menu */}
                    <div className="relative">
                      <button
                        onClick={() =>
                          setActiveMenuPid(activeMenuPid === proc.pid ? null : proc.pid)
                        }
                        className="p-1.5 rounded-lg hover:bg-dark-hover text-slate-400 hover:text-white transition"
                      >
                        <MoreVertical className="w-4 h-4" />
                      </button>

                      {activeMenuPid === proc.pid && (
                        <div className="absolute right-0 rtl:right-auto rtl:left-0 mt-1 w-48 bg-dark-surface border border-dark-border rounded-xl shadow-2xl z-40 py-1 text-xs">
                          <button
                            onClick={() => {
                              openFileLocation(proc.path);
                              setActiveMenuPid(null);
                            }}
                            className="w-full flex items-center gap-2 px-3 py-2 text-slate-300 hover:bg-dark-hover hover:text-white text-left rtl:text-right"
                          >
                            <FolderOpen className="w-3.5 h-3.5 text-brand-cyan" />
                            <span>{t.action_open_explorer}</span>
                          </button>

                          <button
                            onClick={() => {
                              onOpenInspectModal(proc);
                              setActiveMenuPid(null);
                            }}
                            className="w-full flex items-center gap-2 px-3 py-2 text-slate-300 hover:bg-dark-hover hover:text-white text-left rtl:text-right"
                          >
                            <Info className="w-3.5 h-3.5 text-brand-emerald" />
                            <span>{t.action_inspect}</span>
                          </button>

                          <button
                            onClick={() => {
                              onOpenRuleModal(proc);
                              setActiveMenuPid(null);
                            }}
                            className="w-full flex items-center gap-2 px-3 py-2 text-slate-300 hover:bg-dark-hover hover:text-white text-left rtl:text-right"
                          >
                            <Sliders className="w-3.5 h-3.5 text-amber-400" />
                            <span>{t.action_shape_traffic}</span>
                          </button>

                          <div className="my-1 border-t border-dark-border/40" />

                          <button
                            onClick={() => {
                              if (window.confirm(`${t.action_kill_confirm}\n${proc.name}`)) {
                                killProcess(proc.pid);
                              }
                              setActiveMenuPid(null);
                            }}
                            className="w-full flex items-center gap-2 px-3 py-2 text-rose-400 hover:bg-rose-500/10 text-left rtl:text-right"
                          >
                            <Skull className="w-3.5 h-3.5" />
                            <span>{t.action_kill_proc}</span>
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                </div>

                {/* Sub-Rows: Individual Connection Streams */}
                {isExpanded && (
                  <div className="bg-dark-bg/60 pl-8 rtl:pl-0 rtl:pr-8">
                    {proc.streams.length === 0 ? (
                      <div className="py-2.5 px-6 text-slate-500 text-xs italic">
                        No active network sockets open for this process
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
  );
};
