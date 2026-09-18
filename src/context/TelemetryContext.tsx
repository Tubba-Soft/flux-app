import React, { createContext, useContext, useState, useEffect } from 'react';
import { ProcessTraffic, StreamTraffic, DriverStatus, AppRule, StreamRule, NewAppAlert, GlobalTelemetry } from '../types';

export interface RemoteLockInfo {
  isLocked: boolean;
  title: string;
  message: string;
  actionUrl: string;
  actionLabel: string;
}

const APP_VERSION = '1.0.0';
const REMOTE_CONTROL_URL = 'https://raw.githubusercontent.com/Tubba-Soft/flux/main/app-control.json';

function isVersionOlder(current: string, minRequired: string): boolean {
  if (!minRequired) return false;
  const cParts = current.split('.').map((p) => parseInt(p, 10) || 0);
  const mParts = minRequired.split('.').map((p) => parseInt(p, 10) || 0);
  for (let i = 0; i < 3; i++) {
    const c = cParts[i] || 0;
    const m = mParts[i] || 0;
    if (c < m) return true;
    if (c > m) return false;
  }
  return false;
}

interface TelemetryContextType {
  processes: ProcessTraffic[];
  driverStatus: DriverStatus;
  isBrowserPreview: boolean;
  globalDownSpeed: number;
  globalUpSpeed: number;
  globalTotalDown: number;
  globalTotalUp: number;
  todayTotalDown: number;
  todayTotalUp: number;
  todayTotalBytes: number;
  activeSocketsCount: number;
  newAppAlert: NewAppAlert | null;
  remoteLockInfo: RemoteLockInfo;
  clearAlert: () => void;
  updateProcessRule: (rule: AppRule) => Promise<void>;
  updateStreamRule: (rule: StreamRule) => Promise<void>;
  openFileLocation: (path: string) => Promise<void>;
  killProcess: (pid: number) => Promise<void>;
  closeSocket: (local_ip: string, local_port: number, remote_ip: string, remote_port: number) => Promise<void>;
  elevateAdmin: () => Promise<void>;
  autostart: boolean;
  setAutostart: (enable: boolean) => Promise<void>;
  runInBackground: boolean;
  setRunInBackground: (enable: boolean) => Promise<void>;
  taskbarWidget: boolean;
  setTaskbarWidget: (enable: boolean) => Promise<void>;
}

const TelemetryContext = createContext<TelemetryContextType | null>(null);

export const TelemetryProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [processes, setProcesses] = useState<ProcessTraffic[]>([]);
  const [backendGlobal, setBackendGlobal] = useState<GlobalTelemetry | null>(null);
  const [isBrowserPreview, setIsBrowserPreview] = useState(false);
  const [driverStatus, setDriverStatus] = useState<DriverStatus>({
    is_admin: false,
    is_driver_loaded: false,
    mode: 'passive_monitor',
    packet_count: 0,
    diverted_bytes: 0,
    error_message: null,
  });
  const [newAppAlert, setNewAppAlert] = useState<NewAppAlert | null>(null);
  const [autostart, setAutostartState] = useState<boolean>(false);
  const [runInBackground, setRunInBackgroundState] = useState<boolean>(true);
  const [taskbarWidget, setTaskbarWidgetState] = useState<boolean>(true);
  const [remoteLockInfo, setRemoteLockInfo] = useState<RemoteLockInfo>({
    isLocked: false,
    title: '',
    message: '',
    actionUrl: 'https://tubbasoft.com',
    actionLabel: '',
  });

  // Remote Kill-Switch & Version Deprecation Poller
  useEffect(() => {
    const verifyRemoteStatus = async () => {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 4000);
        const res = await fetch(REMOTE_CONTROL_URL, {
          signal: controller.signal,
          cache: 'no-store',
        });
        clearTimeout(timeoutId);

        if (!res.ok) return;
        const data = await res.json();

        const shouldLock =
          data.kill_all === true ||
          (Array.isArray(data.disabled_versions) && data.disabled_versions.includes(APP_VERSION)) ||
          isVersionOlder(APP_VERSION, data.min_required_version);

        if (shouldLock) {
          const lockState: RemoteLockInfo = {
            isLocked: true,
            title: data.notice_title_ar || data.notice_title_en || 'تنبيه من المطور',
            message: data.notice_msg_ar || data.notice_msg_en || 'تم إيقاف هذا الإصدار من قبل المطور.',
            actionUrl: data.action_url || 'https://tubbasoft.com',
            actionLabel: data.action_label_ar || data.action_label_en || 'تحميل التحديث الآن',
          };
          setRemoteLockInfo(lockState);

          try {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('enforce_remote_lockdown', { reason: lockState.message });
          } catch (e) {
            console.warn('Failed to call enforce_remote_lockdown:', e);
          }
        }
      } catch (err) {
        // Offline or unreachable - network monitor continues safely
        console.log('[Flux] Remote control check skipped (offline or unreachable)');
      }
    };

    verifyRemoteStatus();
    const interval = setInterval(verifyRemoteStatus, 30 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    let unlistenTelemetry: (() => void) | undefined;
    let unlistenAlert: (() => void) | undefined;

    const setupTauriListeners = async () => {
      try {
        console.log('[NetFlow] Attempting to connect to Tauri backend...');
        const { listen } = await import('@tauri-apps/api/event');
        const { invoke } = await import('@tauri-apps/api/core');
        console.log('[NetFlow] Tauri API modules loaded successfully.');

        // Initial fetch
        const initialStatus = await invoke<DriverStatus>('get_driver_status');
        console.log('[NetFlow] Driver status:', initialStatus);
        setDriverStatus(initialStatus);

        const isAuto = await invoke<boolean>('get_autostart');
        setAutostartState(isAuto);

        const isBg = await invoke<boolean>('get_run_in_background');
        setRunInBackgroundState(isBg);

        const isWidget = await invoke<boolean>('get_taskbar_widget');
        setTaskbarWidgetState(isWidget);

        const initialSnapshot = await invoke<ProcessTraffic[]>('get_telemetry_snapshot');
        console.log('[NetFlow] Initial snapshot received:', initialSnapshot?.length, 'processes');
        if (initialSnapshot && initialSnapshot.length > 0) {
          setProcesses(initialSnapshot);
        }

        // Listen for live high-frequency telemetry events
        unlistenTelemetry = await listen<ProcessTraffic[]>('telemetry-update', (event) => {
          setProcesses(event.payload || []);
        });
        console.log('[NetFlow] Live telemetry listener registered.');

        // Listen for true kernel-level global telemetry events
        let unlistenGlobal: (() => void) | undefined;
        try {
          unlistenGlobal = await listen<GlobalTelemetry>('global-telemetry', (event) => {
            if (event.payload) {
              setBackendGlobal(event.payload);
            }
          });
        } catch (e) {
          // ignore
        }

        // Listen for new application detected events
        unlistenAlert = await listen<NewAppAlert>('new-app-detected', (event) => {
          setNewAppAlert(event.payload);
        });

        // Periodically refresh driver stats
        const statusTimer = setInterval(async () => {
          try {
            const status = await invoke<DriverStatus>('get_driver_status');
            setDriverStatus(status);
          } catch (e) {
            // ignore
          }
        }, 1500);

        return () => {
          clearInterval(statusTimer);
          if (unlistenGlobal) unlistenGlobal();
        };
      } catch (err) {
        console.error('[NetFlow] Tauri backend connection FAILED:', err);
        setIsBrowserPreview(true);
        loadMockData();
      }
    };

    setupTauriListeners();

    return () => {
      if (unlistenTelemetry) unlistenTelemetry();
      if (unlistenAlert) unlistenAlert();
    };
  }, []);

  const loadMockData = () => {
    const mockInitial: ProcessTraffic[] = [
      {
        pid: 14920,
        name: 'chrome.exe',
        path: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
        down_speed_bps: 4850000,
        up_speed_bps: 240000,
        total_down_bytes: 382900140,
        total_up_bytes: 14092030,
        active_streams_count: 3,
        is_blocked: false,
        down_limit_kbps: null,
        up_limit_kbps: null,
        priority: 'Normal',
        streams: [
          {
            id: 'TCP-192.168.1.100:58102-142.250.190.46:443',
            protocol: 'TCP',
            local_ip: '192.168.1.100',
            local_port: 58102,
            remote_ip: '142.250.190.46',
            remote_port: 443,
            remote_domain: 'youtube.com',
            down_speed_bps: 3600000,
            up_speed_bps: 120000,
            total_down_bytes: 280000000,
            total_up_bytes: 8400000,
            is_blocked: false,
            down_limit_kbps: null,
            up_limit_kbps: null,
            state: 'ESTABLISHED',
          },
          {
            id: 'TCP-192.168.1.100:58104-172.217.16.206:443',
            protocol: 'TCP',
            local_ip: '192.168.1.100',
            local_port: 58104,
            remote_ip: '172.217.16.206',
            remote_port: 443,
            remote_domain: 'fonts.googleapis.com',
            down_speed_bps: 1250000,
            up_speed_bps: 120000,
            total_down_bytes: 102900140,
            total_up_bytes: 5692030,
            is_blocked: false,
            down_limit_kbps: null,
            up_limit_kbps: null,
            state: 'ESTABLISHED',
          },
        ],
      },
      {
        pid: 8840,
        name: 'Discord.exe',
        path: 'C:\\Users\\User\\AppData\\Local\\Discord\\app-1.0.9015\\Discord.exe',
        down_speed_bps: 980000,
        up_speed_bps: 450000,
        total_down_bytes: 84100200,
        total_up_bytes: 42300100,
        active_streams_count: 2,
        is_blocked: false,
        down_limit_kbps: 1500,
        up_limit_kbps: null,
        priority: 'High',
        streams: [
          {
            id: 'UDP-192.168.1.100:50012-66.22.241.9:50001',
            protocol: 'UDP',
            local_ip: '192.168.1.100',
            local_port: 50012,
            remote_ip: '66.22.241.9',
            remote_port: 50001,
            remote_domain: 'voice-webrtc.discord.gg',
            down_speed_bps: 820000,
            up_speed_bps: 420000,
            total_down_bytes: 70000000,
            total_up_bytes: 35000000,
            is_blocked: false,
            down_limit_kbps: null,
            up_limit_kbps: null,
            state: 'ACTIVE',
          },
        ],
      },
      {
        pid: 22100,
        name: 'steam.exe',
        path: 'C:\\Program Files (x86)\\Steam\\steam.exe',
        down_speed_bps: 12400000,
        up_speed_bps: 180000,
        total_down_bytes: 940300100,
        total_up_bytes: 18900000,
        active_streams_count: 4,
        is_blocked: false,
        down_limit_kbps: 15000,
        up_limit_kbps: null,
        priority: 'Normal',
        streams: [
          {
            id: 'TCP-192.168.1.100:54201-162.254.195.34:80',
            protocol: 'TCP',
            local_ip: '192.168.1.100',
            local_port: 54201,
            remote_ip: '162.254.195.34',
            remote_port: 80,
            remote_domain: 'content1.steampowered.com',
            down_speed_bps: 12400000,
            up_speed_bps: 180000,
            total_down_bytes: 940300100,
            total_up_bytes: 18900000,
            is_blocked: false,
            down_limit_kbps: null,
            up_limit_kbps: null,
            state: 'ESTABLISHED',
          },
        ],
      },
      {
        pid: 5412,
        name: 'Spotify.exe',
        path: 'C:\\Users\\User\\AppData\\Roaming\\Spotify\\Spotify.exe',
        down_speed_bps: 320000,
        up_speed_bps: 15000,
        total_down_bytes: 14200000,
        total_up_bytes: 950000,
        active_streams_count: 1,
        is_blocked: false,
        down_limit_kbps: null,
        up_limit_kbps: null,
        priority: 'Normal',
        streams: [
          {
            id: 'TCP-192.168.1.100:51299-35.186.224.25:443',
            protocol: 'TCP',
            local_ip: '192.168.1.100',
            local_port: 51299,
            remote_ip: '35.186.224.25',
            remote_port: 443,
            remote_domain: 'audio-ak-spotify-com.akamaized.net',
            down_speed_bps: 320000,
            up_speed_bps: 15000,
            total_down_bytes: 14200000,
            total_up_bytes: 950000,
            is_blocked: false,
            down_limit_kbps: null,
            up_limit_kbps: null,
            state: 'ESTABLISHED',
          },
        ],
      },
      {
        pid: 1104,
        name: 'svchost.exe',
        path: 'C:\\Windows\\System32\\svchost.exe',
        down_speed_bps: 45000,
        up_speed_bps: 12000,
        total_down_bytes: 4200100,
        total_up_bytes: 1100000,
        active_streams_count: 2,
        is_blocked: false,
        down_limit_kbps: null,
        up_limit_kbps: null,
        priority: 'Low',
        streams: [],
      },
    ];

    setProcesses(mockInitial);

    // Live oscillation for browser test
    const interval = setInterval(() => {
      setProcesses((prev) =>
        prev.map((p) => {
          const jitter = (Math.random() - 0.48) * 0.1;
          const newDown = Math.max(0, Math.round(p.down_speed_bps * (1 + jitter)));
          const newUp = Math.max(0, Math.round(p.up_speed_bps * (1 + jitter)));
          return {
            ...p,
            down_speed_bps: newDown,
            up_speed_bps: newUp,
            total_down_bytes: p.total_down_bytes + Math.round(newDown * 0.3),
            total_up_bytes: p.total_up_bytes + Math.round(newUp * 0.3),
          };
        })
      );
    }, 400);

    return () => clearInterval(interval);
  };

  const clearAlert = () => setNewAppAlert(null);

  const updateProcessRule = async (rule: AppRule) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_process_rule', { rule });
    } catch (e) {
      console.warn('Backend unavailable, updating local mock state', e);
    }
    setProcesses((prev) =>
      prev.map((p) => {
        if (p.path === rule.path) {
          return {
            ...p,
            is_blocked: rule.is_blocked,
            down_limit_kbps: rule.down_limit_kbps,
            up_limit_kbps: rule.up_limit_kbps,
            priority: rule.priority,
          };
        }
        return p;
      })
    );
  };

  const updateStreamRule = async (rule: StreamRule) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_stream_rule', { rule });
    } catch (e) {
      console.warn('Backend unavailable, updating local mock state', e);
    }
    setProcesses((prev) =>
      prev.map((p) => ({
        ...p,
        streams: p.streams.map((s) => {
          if (s.id === rule.stream_id) {
            return {
              ...s,
              is_blocked: rule.is_blocked,
              down_limit_kbps: rule.down_limit_kbps,
              up_limit_kbps: rule.up_limit_kbps,
            };
          }
          return s;
        }),
      }))
    );
  };

  const openFileLocation = async (path: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('open_file_location', { path });
    } catch (e) {
      alert(`Open file location: ${path}`);
    }
  };

  const killProcess = async (pid: number) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('terminate_process', { pid });
    } catch (e) {
      console.warn('Backend unavailable, killing in local state', e);
    }
    setProcesses((prev) => prev.filter((p) => p.pid !== pid));
  };

  const closeSocket = async (local_ip: string, local_port: number, remote_ip: string, remote_port: number) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('close_stream_socket', { localIp: local_ip, localPort: local_port, remoteIp: remote_ip, remotePort: remote_port });
    } catch (e) {
      console.warn('Backend unavailable, closing in local state', e);
    }
  };

  const elevateAdmin = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('request_admin_elevation');
    } catch (e) {
      alert('Elevate to administrator requires running on Windows');
    }
  };

  const setAutostart = async (enable: boolean) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_autostart', { enabled: enable });
      setAutostartState(enable);
    } catch (e) {
      console.error('Failed to set autostart', e);
      // Revert/refresh state from backend to reflect true status
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const trueStatus = await invoke<boolean>('get_autostart');
        setAutostartState(trueStatus);
      } catch (_) {}
    }
  };

  const setRunInBackground = async (enable: boolean) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_run_in_background', { enabled: enable });
      setRunInBackgroundState(enable);
    } catch (e) {
      console.error('Failed to set run_in_background', e);
    }
  };

  const setTaskbarWidget = async (enable: boolean) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('toggle_taskbar_widget', { enabled: enable });
      setTaskbarWidgetState(enable);
    } catch (e) {
      console.warn('Failed to toggle taskbar widget', e);
      setTaskbarWidgetState(enable);
    }
  };

  // Compute live global metrics
  const sumDownSpeed = processes.reduce((acc, p) => acc + (p.down_speed_bps || 0), 0);
  const sumUpSpeed = processes.reduce((acc, p) => acc + (p.up_speed_bps || 0), 0);
  const sumTotalDown = processes.reduce((acc, p) => acc + (p.total_down_bytes || 0), 0);
  const sumTotalUp = processes.reduce((acc, p) => acc + (p.total_up_bytes || 0), 0);

  const globalDownSpeed = backendGlobal ? Math.max(backendGlobal.down_speed_bps, sumDownSpeed) : sumDownSpeed;
  const globalUpSpeed = backendGlobal ? Math.max(backendGlobal.up_speed_bps, sumUpSpeed) : sumUpSpeed;
  const globalTotalDown = backendGlobal ? Math.max(backendGlobal.total_down_bytes, sumTotalDown) : sumTotalDown;
  const globalTotalUp = backendGlobal ? Math.max(backendGlobal.total_up_bytes, sumTotalUp) : sumTotalUp;
  const todayTotalDown = backendGlobal ? backendGlobal.today_down_bytes : globalTotalDown;
  const todayTotalUp = backendGlobal ? backendGlobal.today_up_bytes : globalTotalUp;
  const todayTotalBytes = todayTotalDown + todayTotalUp;
  const activeSocketsCount = processes.reduce((acc, p) => acc + p.streams.length, 0);

  return (
    <TelemetryContext.Provider
      value={{
        processes,
        driverStatus,
        isBrowserPreview,
        globalDownSpeed,
        globalUpSpeed,
        globalTotalDown,
        globalTotalUp,
        todayTotalDown,
        todayTotalUp,
        todayTotalBytes,
        activeSocketsCount,
        newAppAlert,
        remoteLockInfo,
        clearAlert,
        updateProcessRule,
        updateStreamRule,
        openFileLocation,
        killProcess,
        closeSocket,
        elevateAdmin,
        autostart,
        setAutostart,
        runInBackground,
        setRunInBackground,
        taskbarWidget,
        setTaskbarWidget,
      }}
    >
      {children}
    </TelemetryContext.Provider>
  );
};

export const useTelemetry = () => {
  const ctx = useContext(TelemetryContext);
  if (!ctx) throw new Error('useTelemetry must be used within TelemetryProvider');
  return ctx;
};
