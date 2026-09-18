import React, { useState } from 'react';
import { StreamTraffic } from '../types';
import { useLanguage } from '../context/LanguageContext';
import { useTelemetry } from '../context/TelemetryContext';
import { formatSpeed, formatBytes } from '../utils/formatters';
import {
  Globe,
  Sliders,
  Shield,
  ShieldOff,
  XCircle,
  ArrowDown,
  ArrowUp,
  Radio
} from 'lucide-react';

interface Props {
  stream: StreamTraffic;
  onOpenRuleModal: (stream: StreamTraffic) => void;
}

export const StreamRow: React.FC<Props> = ({ stream, onOpenRuleModal }) => {
  const { t, isRtl } = useLanguage();
  const { updateStreamRule, closeSocket } = useTelemetry();
  const [isClosing, setIsClosing] = useState(false);

  const handleToggleBlock = (e: React.MouseEvent) => {
    e.stopPropagation();
    updateStreamRule({
      stream_id: stream.id,
      is_blocked: !stream.is_blocked,
      down_limit_kbps: stream.down_limit_kbps,
      up_limit_kbps: stream.up_limit_kbps,
    });
  };

  const handleCloseSocket = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (window.confirm(t.stream_close_confirm)) {
      setIsClosing(true);
      await closeSocket(stream.local_ip, stream.local_port, stream.remote_ip, stream.remote_port);
      setIsClosing(false);
    }
  };

  const isTcp = stream.protocol.toUpperCase() === 'TCP';

  return (
    <div className="grid grid-cols-[minmax(260px,1.4fr)_130px_110px_110px_110px_140px_50px_40px] items-center px-6 py-2 text-xs border-b border-transparent dark:border-slate-800/25 bg-slate-50/50 hover:bg-slate-100 dark:bg-[#080D18]/60 dark:hover:bg-[#131D31] transition-colors duration-150">
      {/* 1. Protocol + Endpoints + Domain */}
      <div className="flex items-center gap-2.5 min-w-0 pl-7 rtl:pl-0 rtl:pr-7">
        {/* Protocol Badge */}
        <span
          className={`px-1.5 py-0.5 rounded font-mono font-bold text-[10px] shrink-0 ${
            isTcp
              ? 'bg-purple-100 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 border border-transparent dark:border-purple-800/40'
              : 'bg-cyan-100 dark:bg-cyan-950/60 text-cyan-700 dark:text-cyan-300 border border-transparent dark:border-cyan-800/40'
          }`}
        >
          {stream.protocol}
        </span>

        {/* 5-Tuple Endpoints */}
        <div className="flex items-center gap-1 font-mono text-[11px] text-slate-600 dark:text-slate-300 truncate">
          <span className="shrink-0">{stream.local_ip}:{stream.local_port}</span>
          <span className="text-slate-400 dark:text-slate-500 shrink-0">→</span>
          <span className="text-slate-800 dark:text-slate-200 font-semibold truncate">{stream.remote_ip}:{stream.remote_port}</span>
        </div>

        {/* Domain / SNI */}
        {stream.remote_domain && (
          <div className="hidden sm:flex items-center gap-1 px-1.5 py-0.5 rounded bg-blue-50 dark:bg-blue-950/40 border border-transparent dark:border-blue-800/30 text-blue-700 dark:text-blue-300 text-[10px] shrink-0 font-sans">
            <Globe className="w-3 h-3 text-blue-500 dark:text-blue-400" />
            <span className="truncate max-w-[120px]" title={stream.remote_domain}>
              {stream.remote_domain}
            </span>
          </div>
        )}
      </div>

      {/* 2. Connection State */}
      <div className="text-center">
        <span className="text-[10px] text-slate-400 dark:text-slate-500 font-mono uppercase">
          {stream.state}
        </span>
      </div>

      {/* 3. Down Speed */}
      <div className="flex items-center justify-center gap-1 font-mono text-xs px-1">
        <ArrowDown className="w-3 h-3 text-emerald-600 dark:text-emerald-400 shrink-0" />
        <span className={stream.down_speed_bps > 0 ? 'text-emerald-600 dark:text-emerald-400 font-semibold' : 'text-slate-400 dark:text-slate-600'}>
          {formatSpeed(stream.down_speed_bps)}
        </span>
      </div>

      {/* 4. Up Speed */}
      <div className="flex items-center justify-center gap-1 font-mono text-xs px-1">
        <ArrowUp className="w-3 h-3 text-cyan-600 dark:text-cyan-400 shrink-0" />
        <span className={stream.up_speed_bps > 0 ? 'text-cyan-600 dark:text-cyan-400 font-semibold' : 'text-slate-400 dark:text-slate-600'}>
          {formatSpeed(stream.up_speed_bps)}
        </span>
      </div>

      {/* 5. Total Consumed */}
      <div className="text-center font-mono text-xs text-slate-500 dark:text-slate-400">
        {formatBytes(stream.total_down_bytes + stream.total_up_bytes)}
      </div>

      {/* 6. Controls: Limit Button */}
      <div className="flex justify-center" onClick={(e) => e.stopPropagation()}>
        <button
          onClick={() => onOpenRuleModal(stream)}
          className={`w-full max-w-[130px] px-2.5 py-1 rounded-lg text-xs font-medium flex items-center justify-center gap-1.5 transition truncate ${
            stream.down_limit_kbps || stream.up_limit_kbps
              ? 'bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-transparent dark:border-amber-500/40 font-semibold'
              : 'bg-slate-200/80 dark:bg-slate-800/80 hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 border border-transparent dark:border-slate-700/60 shadow-sm dark:shadow-none'
          }`}
          title={t.action_shape_traffic}
        >
          <Sliders className="w-3 h-3 shrink-0" />
          <span className="truncate">{stream.down_limit_kbps ? `${stream.down_limit_kbps} KB/s` : t.rule_no_limit}</span>
        </button>
      </div>

      {/* 7. Block Button */}
      <div className="flex justify-center" onClick={(e) => e.stopPropagation()}>
        <button
          onClick={handleToggleBlock}
          className={`p-1.5 rounded-lg border transition ${
            stream.is_blocked
              ? 'bg-rose-500/15 border-transparent dark:border-rose-500/50 text-rose-600 dark:text-rose-400'
              : 'bg-slate-200/80 dark:bg-slate-800/80 border-transparent dark:border-slate-700/60 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-300 dark:hover:bg-slate-700 shadow-sm dark:shadow-none'
          }`}
          title={t.col_block}
        >
          {stream.is_blocked ? <ShieldOff className="w-3.5 h-3.5 text-rose-600 dark:text-rose-400" /> : <Shield className="w-3.5 h-3.5" />}
        </button>
      </div>

      {/* 8. Close Socket (TCP only) */}
      <div className="flex justify-center" onClick={(e) => e.stopPropagation()}>
        {isTcp ? (
          <button
            onClick={handleCloseSocket}
            disabled={isClosing}
            className="p-1.5 rounded-lg hover:bg-rose-500/10 text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 transition"
            title={t.stream_close_socket}
          >
            <XCircle className="w-4 h-4" />
          </button>
        ) : (
          <span className="w-4 h-4" />
        )}
      </div>
    </div>
  );
};
