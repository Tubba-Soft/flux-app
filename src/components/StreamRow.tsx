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
    <div className="flex items-center justify-between py-2 px-6 border-t border-dark-border/30 bg-dark-bg/40 hover:bg-dark-card/50 transition text-xs">
      {/* Protocol + Endpoints + Domain */}
      <div className="flex items-center gap-3 min-w-[320px] flex-1">
        {/* Protocol Badge */}
        <span
          className={`px-2 py-0.5 rounded font-mono font-bold text-[10px] ${
            isTcp
              ? 'bg-purple-950/60 text-purple-300 border border-purple-800/40'
              : 'bg-cyan-950/60 text-cyan-300 border border-cyan-800/40'
          }`}
        >
          {stream.protocol}
        </span>

        {/* 5-Tuple Endpoints */}
        <div className="flex items-center gap-1.5 font-mono text-slate-300">
          <span>{stream.local_ip}:{stream.local_port}</span>
          <span className="text-slate-500">→</span>
          <span className="text-slate-200 font-semibold">{stream.remote_ip}:{stream.remote_port}</span>
        </div>

        {/* Domain / SNI */}
        {stream.remote_domain && (
          <div className="flex items-center gap-1 px-2 py-0.5 rounded bg-blue-950/40 border border-blue-800/30 text-blue-300 text-[11px] font-sans">
            <Globe className="w-3 h-3 text-blue-400" />
            <span className="truncate max-w-[200px]" title={stream.remote_domain}>
              {stream.remote_domain}
            </span>
          </div>
        )}

        {/* State */}
        <span className="text-[10px] text-slate-500 font-mono uppercase">
          {stream.state}
        </span>
      </div>

      {/* Speeds */}
      <div className="flex items-center gap-8 min-w-[280px]">
        {/* Down speed */}
        <div className="flex items-center gap-1 w-24 text-right">
          <ArrowDown className="w-3 h-3 text-brand-emerald shrink-0" />
          <span className={`font-mono ${stream.down_speed_bps > 0 ? 'text-brand-emerald font-semibold' : 'text-slate-500'}`}>
            {formatSpeed(stream.down_speed_bps)}
          </span>
        </div>

        {/* Up speed */}
        <div className="flex items-center gap-1 w-24 text-right">
          <ArrowUp className="w-3 h-3 text-brand-cyan shrink-0" />
          <span className={`font-mono ${stream.up_speed_bps > 0 ? 'text-brand-cyan font-semibold' : 'text-slate-500'}`}>
            {formatSpeed(stream.up_speed_bps)}
          </span>
        </div>

        {/* Total consumed */}
        <div className="w-20 text-slate-400 font-mono text-[11px] text-right">
          {formatBytes(stream.total_down_bytes + stream.total_up_bytes)}
        </div>
      </div>

      {/* Controls: Throttle, Block, Close */}
      <div className="flex items-center gap-2 pl-4">
        {/* Limit Button */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onOpenRuleModal(stream);
          }}
          className={`px-2 py-1 rounded flex items-center gap-1 transition ${
            stream.down_limit_kbps || stream.up_limit_kbps
              ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40 font-semibold'
              : 'bg-dark-card hover:bg-dark-hover text-slate-400 border border-dark-border'
          }`}
          title={t.action_shape_traffic}
        >
          <Sliders className="w-3 h-3" />
          {stream.down_limit_kbps ? `${stream.down_limit_kbps} KB/s` : t.rule_no_limit}
        </button>

        {/* Block Button */}
        <button
          onClick={handleToggleBlock}
          className={`p-1 rounded transition ${
            stream.is_blocked
              ? 'bg-rose-500/20 text-rose-400 border border-rose-500/40'
              : 'hover:bg-dark-hover text-slate-500 hover:text-slate-300'
          }`}
          title={t.col_block}
        >
          {stream.is_blocked ? <ShieldOff className="w-3.5 h-3.5 text-rose-400" /> : <Shield className="w-3.5 h-3.5" />}
        </button>

        {/* Close Socket (TCP only) */}
        {isTcp && (
          <button
            onClick={handleCloseSocket}
            disabled={isClosing}
            className="p-1 rounded hover:bg-rose-500/10 text-slate-500 hover:text-rose-400 transition"
            title={t.stream_close_socket}
          >
            <XCircle className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
    </div>
  );
};
