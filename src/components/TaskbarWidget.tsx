import React, { useState, useEffect, useRef, useCallback } from 'react';
import { ArrowDown, ArrowUp, Lock, GripVertical, ExternalLink } from 'lucide-react';
import { getCurrentWebviewWindow, WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { PhysicalPosition } from '@tauri-apps/api/dpi';
import { formatSpeed } from '../utils/formatters';
import { GlobalTelemetry, WidgetConfig } from '../types';

export const TaskbarWidget: React.FC = () => {
  const [downSpeed, setDownSpeed] = useState<number>(0);
  const [upSpeed, setUpSpeed] = useState<number>(0);
  const [isTranslucent, setIsTranslucent] = useState<boolean>(false);
  const [showMenu, setShowMenu] = useState<boolean>(false);

  const [config, setConfig] = useState<WidgetConfig>({
    locked: false,
    auto_transparency: true,
    transparency_delay_secs: 2,
    transparency_opacity: 30,
    color_theme: 'emerald_cyan',
    preset: 'bottom-right',
  });

  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const savePosDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pointerDownPos = useRef<{ x: number; y: number } | null>(null);
  const isDraggingRef = useRef<boolean>(false);
  const isHoveredRef = useRef<boolean>(false); // تتبع دقيق للهوفر يمنع التلاشي الخاطئ
  const menuRef = useRef<HTMLDivElement | null>(null);
  const configRef = useRef(config);
  configRef.current = config;
  const showMenuRef = useRef(showMenu);
  showMenuRef.current = showMenu;

  // إيقاظ الودجت وإلغاء أي مؤقت شفافية
  const wakeUpFromIdle = useCallback(() => {
    if (idleTimerRef.current) {
      clearTimeout(idleTimerRef.current);
      idleTimerRef.current = null;
    }
    setIsTranslucent(false);
  }, []);

  // بدء مؤقت التلاشي حصرياً عند زوال المؤشر
  const scheduleIdleTimer = useCallback(() => {
    if (idleTimerRef.current) {
      clearTimeout(idleTimerRef.current);
      idleTimerRef.current = null;
    }

    // شرط صارم: لا شفافية إذا كان الماوس فوق الودجت أو القائمة مفتوحة أو أثناء السحب
    if (
      configRef.current.auto_transparency &&
      !isHoveredRef.current &&
      !showMenuRef.current &&
      !isDraggingRef.current
    ) {
      idleTimerRef.current = setTimeout(() => {
        setIsTranslucent(true);
      }, configRef.current.transparency_delay_secs * 1000);
    }
  }, []);

  // تفاعل الدخول والخروج للماوس
  const handleMouseEnter = () => {
    isHoveredRef.current = true;
    wakeUpFromIdle();
  };

  const handleMouseLeave = () => {
    isHoveredRef.current = false;
    scheduleIdleTimer();
  };

  // فتح وإظهار التطبيق الرئيسي
  const handleOpenApp = useCallback(async () => {
    try {
      const mainWin = await WebviewWindow.getByLabel('main');
      if (mainWin) {
        await mainWin.show();
        await mainWin.unminimize();
        await mainWin.setFocus();
      }
    } catch (e) {
      console.error('Failed to open main app window:', e);
    }
  }, []);

  // تبديل حالة القفل وحفظها
  const toggleLock = useCallback(async () => {
    const updated = { ...configRef.current, locked: !configRef.current.locked };
    setConfig(updated);
    try {
      await invoke('save_widget_config', { config: updated });
    } catch (e) {
      console.error('Failed to save config:', e);
    }
    setShowMenu(false);
  }, []);

  // الاستماع المباشر للتحديثات الفورية للإعدادات من النافذة الرئيسية دون الحاجة لإعادة تشغيل البرنامج
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let isMounted = true;

    listen<WidgetConfig>('widget-config-changed', (event) => {
      if (isMounted && event.payload) {
        setConfig(event.payload);
        configRef.current = event.payload;
        wakeUpFromIdle();
        scheduleIdleTimer();
      }
    }).then((fn) => {
      if (isMounted) unlisten = fn;
      else fn();
    }).catch((e) => console.warn('Config change listen error:', e));

    return () => {
      isMounted = false;
      if (unlisten) unlisten();
    };
  }, [wakeUpFromIdle, scheduleIdleTimer]);

  // التهيئة المبدئية ومراقبة حركة النافذة لحفظ الإحداثيات
  useEffect(() => {
    let unlistenMove: UnlistenFn | undefined;
    let isMounted = true;

    const init = async () => {
      try {
        // جلب الإعدادات والسرعة الأولية معاً فوراً دون انتظار نبضة الـ Telemetry الأولى
        const [savedConfig, initialTelemetry] = await Promise.all([
          invoke<WidgetConfig>('get_widget_config').catch(() => null),
          invoke<GlobalTelemetry>('get_global_telemetry').catch(() => null),
        ]);

        if (isMounted && savedConfig) setConfig(savedConfig);
        if (isMounted && initialTelemetry) {
          setDownSpeed(initialTelemetry.down_speed_bps);
          setUpSpeed(initialTelemetry.up_speed_bps);
        }

        const currentWin = getCurrentWebviewWindow();

        // استخدام الإحداثيات المباشرة من Payload دون استدعاء IPC زائد
        unlistenMove = await currentWin.listen<PhysicalPosition>('tauri://move', (event) => {
          if (savePosDebounceRef.current) clearTimeout(savePosDebounceRef.current);

          savePosDebounceRef.current = setTimeout(async () => {
            const pos = event.payload;
            if (pos) {
              await invoke('save_widget_position', { x: pos.x, y: pos.y }).catch(() => {});
            }
            isDraggingRef.current = false;
            // فحص الهوفر قبل بدء المؤقت لمنع التلاشي والماوس فوق النافذة
            if (!isHoveredRef.current) {
              scheduleIdleTimer();
            }
          }, 400);
        });

        if (!isMounted && unlistenMove) unlistenMove();
      } catch (e) {
        console.warn('Widget initialization error:', e);
      }
    };

    init();
    scheduleIdleTimer();

    return () => {
      isMounted = false;
      if (unlistenMove) unlistenMove();
      if (savePosDebounceRef.current) clearTimeout(savePosDebounceRef.current);
      if (idleTimerRef.current) clearTimeout(idleTimerRef.current);
    };
  }, [scheduleIdleTimer]);

  // الاستماع للتيليمتري اللحظي
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let isMounted = true;

    listen<GlobalTelemetry>('global-telemetry', (event) => {
      if (isMounted && event.payload) {
        setDownSpeed(event.payload.down_speed_bps);
        setUpSpeed(event.payload.up_speed_bps);
      }
    }).then((fn) => {
      if (isMounted) unlisten = fn;
      else fn();
    }).catch((e) => console.warn('Telemetry listen error:', e));

    return () => {
      isMounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  // إغلاق القائمة عند النقر في الخارج
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowMenu(false);
      }
    };

    if (showMenu) {
      window.addEventListener('mousedown', handleOutsideClick);
    } else {
      if (!isHoveredRef.current) {
        scheduleIdleTimer();
      }
    }

    return () => window.removeEventListener('mousedown', handleOutsideClick);
  }, [showMenu, scheduleIdleTimer]);

  // إدارة تتبع السحب العالمية مع آلية الإطلاق المزدوجة (Native Drag Command + StartDragging)
  useEffect(() => {
    const handleGlobalMouseMove = async (e: MouseEvent) => {
      if (!pointerDownPos.current || configRef.current.locked || isDraggingRef.current) return;

      const deltaX = Math.abs(e.clientX - pointerDownPos.current.x);
      const deltaY = Math.abs(e.clientY - pointerDownPos.current.y);

      // حد 5 بكسلات يمنع إلغاء النقر المزدوج العفوي
      if (deltaX > 5 || deltaY > 5) {
        isDraggingRef.current = true;
        pointerDownPos.current = null;
        wakeUpFromIdle();

        // استدعاء أمر السحب في الـ Rust مباشرة مع fallback إلى API النافذة
        try {
          await invoke('start_widget_drag');
        } catch {
          try {
            await getCurrentWebviewWindow().startDragging();
          } catch (err) {
            console.warn('startDragging error:', err);
          }
        }
      }
    };

    const handleGlobalMouseUp = () => {
      pointerDownPos.current = null;
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        if (!isHoveredRef.current) {
          scheduleIdleTimer();
        }
      }
    };

    window.addEventListener('mousemove', handleGlobalMouseMove);
    window.addEventListener('mouseup', handleGlobalMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleGlobalMouseMove);
      window.removeEventListener('mouseup', handleGlobalMouseUp);
    };
  }, [wakeUpFromIdle, scheduleIdleTimer]);

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0 || showMenu) return;
    pointerDownPos.current = { x: e.clientX, y: e.clientY };
  };

  const getThemeColors = () => {
    switch (config.color_theme) {
      case 'purple_pink':
        return { down: 'text-fuchsia-400', up: 'text-purple-400', border: 'border-fuchsia-500/50 hover:border-fuchsia-400/80' };
      case 'amber_orange':
        return { down: 'text-amber-400', up: 'text-orange-400', border: 'border-amber-500/50 hover:border-amber-400/80' };
      case 'blue_sky':
        return { down: 'text-sky-400', up: 'text-blue-400', border: 'border-sky-500/50 hover:border-sky-400/80' };
      case 'silver_mono':
        return { down: 'text-slate-100', up: 'text-slate-300', border: 'border-slate-500/50 hover:border-slate-300/80' };
      default:
        return { down: 'text-emerald-400', up: 'text-cyan-400', border: 'border-slate-700/80 hover:border-cyan-500/80' };
    }
  };

  const theme = getThemeColors();
  const currentOpacity = isTranslucent && !showMenu ? (config.transparency_opacity ?? 30) / 100 : 1;

  return (
    <div
      onMouseEnter={handleMouseEnter}
      onMouseMove={wakeUpFromIdle}
      onMouseLeave={handleMouseLeave}
      onMouseDown={handleMouseDown}
      onDoubleClick={handleOpenApp}
      onContextMenu={(e) => {
        e.preventDefault();
        wakeUpFromIdle();
        setShowMenu((prev) => !prev);
      }}
      className={`relative w-full h-full flex items-center justify-between px-3 bg-slate-950/90 text-[11px] font-mono select-none tracking-tight transition-opacity duration-200 border ${
        theme.border
      } rounded-full backdrop-blur-xl ${
        config.locked ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'
      }`}
      style={{
        opacity: currentOpacity,
        boxShadow: '0 2px 10px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.12)',
      }}
      title="اسحب للتحريك | انقر مزدوجاً لفتح التطبيق | انقر باليمين للقائمة"
    >
      {/* مؤشر القفل أو التحريك */}
      <div className="flex items-center shrink-0 mr-1 pointer-events-none text-slate-500">
        {config.locked ? (
          <Lock className="w-2.5 h-2.5 text-slate-500/60" />
        ) : (
          <GripVertical className="w-2.5 h-3.5 text-slate-500/60" />
        )}
      </div>

      {/* سرعة التنزيل مع عرض ثابت يمنع التمدد المفاجئ */}
      <div className={`flex items-center gap-1 ${theme.down} font-bold min-w-[78px] shrink-0 pointer-events-none`}>
        <ArrowDown className="w-3 h-3 stroke-[2.5]" />
        <span className="tabular-nums tracking-tight whitespace-nowrap">{formatSpeed(downSpeed)}</span>
      </div>

      {/* فاصل وسطي */}
      <div className="h-3 w-[1px] bg-slate-700/60 mx-1 shrink-0 pointer-events-none" />

      {/* سرعة الرفع مع عرض ثابت يمنع التمدد المفاجئ */}
      <div className={`flex items-center gap-1 ${theme.up} font-bold min-w-[78px] shrink-0 pointer-events-none`}>
        <ArrowUp className="w-3 h-3 stroke-[2.5]" />
        <span className="tabular-nums tracking-tight whitespace-nowrap">{formatSpeed(upSpeed)}</span>
      </div>

      {/* القائمة السريعة مع حماية كاملة من تسرب الأحداث */}
      {showMenu && (
        <div
          ref={menuRef}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => e.stopPropagation()}
          onDoubleClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.stopPropagation()}
          className="absolute inset-0 bg-slate-950/95 rounded-full flex items-center justify-around px-2 z-50 border border-slate-700"
        >
          <button
            onClick={handleOpenApp}
            className="flex items-center gap-1 text-[10px] text-cyan-400 hover:text-cyan-300 font-sans transition"
            title="فتح التطبيق"
          >
            <ExternalLink className="w-3 h-3" />
            <span>فتح</span>
          </button>

          <div className="h-2.5 w-[1px] bg-slate-800" />

          <button
            onClick={toggleLock}
            className="flex items-center gap-1 text-[10px] text-slate-300 hover:text-white font-sans transition"
            title={config.locked ? 'إلغاء التثبيت' : 'تثبيت'}
          >
            <Lock className="w-3 h-3" />
            <span>{config.locked ? 'تحريك' : 'تثبيت'}</span>
          </button>

          <div className="h-2.5 w-[1px] bg-slate-800" />

          <button
            onClick={() => setShowMenu(false)}
            className="text-[10px] text-slate-500 hover:text-slate-300 px-1 font-sans"
          >
            ✕
          </button>
        </div>
      )}
    </div>
  );
};