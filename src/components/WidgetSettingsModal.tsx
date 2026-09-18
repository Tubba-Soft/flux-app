import React, { useState, useEffect } from 'react';
import { Sliders, Lock, Move, Palette, Clock, Check, X, Compass } from 'lucide-react';
import { useLanguage } from '../context/LanguageContext';
import { WidgetConfig } from '../types';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export const COLOR_THEMES = [
  {
    id: 'emerald_cyan',
    nameAr: 'الزمردي والسماوي (الافتراضي)',
    nameEn: 'Emerald & Cyan (Default)',
    downColor: 'text-emerald-400',
    upColor: 'text-cyan-400',
    borderHover: 'border-emerald-500/40',
    previewDown: '#34d399',
    previewUp: '#22d3ee',
  },
  {
    id: 'purple_pink',
    nameAr: 'البنفسجي والنيون الوردي',
    nameEn: 'Neon Purple & Pink',
    downColor: 'text-fuchsia-400',
    upColor: 'text-purple-400',
    borderHover: 'border-fuchsia-500/40',
    previewDown: '#e879f9',
    previewUp: '#c084fc',
  },
  {
    id: 'amber_orange',
    nameAr: 'العنبري والبرتقالي المتوهج',
    nameEn: 'Amber & Glowing Orange',
    downColor: 'text-amber-400',
    upColor: 'text-orange-400',
    borderHover: 'border-amber-500/40',
    previewDown: '#fbbf24',
    previewUp: '#fb923c',
  },
  {
    id: 'blue_sky',
    nameAr: 'الأزرق الكهربائي والسماوي',
    nameEn: 'Electric Blue & Sky',
    downColor: 'text-sky-400',
    upColor: 'text-blue-400',
    borderHover: 'border-sky-500/40',
    previewDown: '#38bdf8',
    previewUp: '#60a5fa',
  },
  {
    id: 'silver_mono',
    nameAr: 'الفضي والرمادي العصري (Monochrome)',
    nameEn: 'Monochrome Modern Silver',
    downColor: 'text-slate-100',
    upColor: 'text-slate-300',
    borderHover: 'border-slate-500/40',
    previewDown: '#f1f5f9',
    previewUp: '#cbd5e1',
  },
];

export const PRESET_POSITIONS = [
  { id: 'bottom-right', labelAr: 'أسفل اليمين (فوق شريط المهام)', labelEn: 'Bottom Right (Taskbar Dock)' },
  { id: 'bottom-center', labelAr: 'منتصف الشاشة بالأسفل', labelEn: 'Bottom Center' },
  { id: 'bottom-left', labelAr: 'أسفل اليسار', labelEn: 'Bottom Left' },
  { id: 'top-right', labelAr: 'أعلى اليمين', labelEn: 'Top Right' },
  { id: 'top-center', labelAr: 'منتصف الشاشة بالأعلى', labelEn: 'Top Center' },
  { id: 'top-left', labelAr: 'أعلى اليسار', labelEn: 'Top Left' },
];

export const WidgetSettingsModal: React.FC<Props> = ({ isOpen, onClose }) => {
  const { isRtl } = useLanguage();

  const [config, setConfig] = useState<WidgetConfig>({
    locked: false,
    auto_transparency: true,
    transparency_delay_secs: 2,
    transparency_opacity: 30,
    color_theme: 'emerald_cyan',
    preset: 'bottom-right',
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke<WidgetConfig>('get_widget_config');
        if (res) setConfig(res);
      } catch (e) {
        // ignore
      }
    };
    if (isOpen) {
      loadConfig();
    }
  }, [isOpen]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_widget_config', { config });
      if (config.locked) {
        await invoke('set_widget_preset', { preset: config.preset });
      }
      onClose();
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  const handlePresetChange = async (presetId: string) => {
    setConfig((prev) => ({ ...prev, preset: presetId }));
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_widget_preset', { preset: presetId });
    } catch (e) {
      // ignore
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-slate-950/80 backdrop-blur-md p-4 animate-fade-in"
      onClick={onClose}
    >
      <div
        className="bg-slate-900/95 border border-slate-700/80 rounded-2xl max-w-lg w-full shadow-2xl relative text-slate-100 flex flex-col max-h-[85vh] ring-1 ring-white/10 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
        dir={isRtl ? 'rtl' : 'ltr'}
      >
        {/* Header (Fixed) */}
        <div className="flex items-center justify-between p-5 pb-4 border-b border-slate-800 shrink-0 bg-slate-900/95">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-cyan/15 text-brand-cyan border border-brand-cyan/30">
              <Sliders className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-bold text-white">
                {isRtl ? 'تخصيص وإدارة ودجت شريط المهام' : 'Taskbar Widget Settings'}
              </h3>
              <p className="text-xs text-slate-400">
                {isRtl ? 'التحكم في التثبيت، الشفافية الذكية، ونظام الألوان' : 'Manage docking presets, smart transparency, and theme colors'}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Scrollable Body with Sleek Scrollbar */}
        <div className="p-5 overflow-y-auto flex-1 min-h-0 space-y-5">
          {/* Section 1: Movement & Pinning */}
          <div className="space-y-3">
          <label className="text-xs font-bold text-slate-300 flex items-center gap-2">
            <Compass className="w-4 h-4 text-brand-cyan" />
            <span>{isRtl ? 'حالة التموضع والتحريك:' : 'Positioning Mode:'}</span>
          </label>

          <div className="grid grid-cols-2 gap-2.5">
            <button
              type="button"
              onClick={() => setConfig((prev) => ({ ...prev, locked: false }))}
              className={`flex items-center gap-2.5 p-3 rounded-xl border text-xs font-semibold transition ${
                !config.locked
                  ? 'bg-brand-cyan/15 border-brand-cyan text-white shadow-sm'
                  : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:bg-slate-800/40'
              }`}
            >
              <Move className="w-4 h-4 text-cyan-400" />
              <div className="text-start">
                <span className="block">{isRtl ? 'سحب وتحريك حر' : 'Free Dragging'}</span>
                <span className="text-[10px] font-normal text-slate-400 block">
                  {isRtl ? 'اسحب الودجت بسهولة لأي مكان' : 'Drag easily to any screen spot'}
                </span>
              </div>
            </button>

            <button
              type="button"
              onClick={() => setConfig((prev) => ({ ...prev, locked: true }))}
              className={`flex items-center gap-2.5 p-3 rounded-xl border text-xs font-semibold transition ${
                config.locked
                  ? 'bg-brand-cyan/15 border-brand-cyan text-white shadow-sm'
                  : 'bg-slate-950/60 border-slate-800 text-slate-400 hover:bg-slate-800/40'
              }`}
            >
              <Lock className="w-4 h-4 text-amber-400" />
              <div className="text-start">
                <span className="block">{isRtl ? 'تثبيت في موقع محدد' : 'Locked / Pinned'}</span>
                <span className="text-[10px] font-normal text-slate-400 block">
                  {isRtl ? 'تثبيت دقيق بدون تحريك بالخطأ' : 'Snap to screen positions'}
                </span>
              </div>
            </button>
          </div>

          {/* Preset Positions (Shown when locked) */}
          {config.locked && (
            <div className="mt-2.5 p-3 rounded-xl bg-slate-950/80 border border-slate-800 space-y-2 animate-fade-in">
              <span className="text-[11px] font-semibold text-slate-300 block">
                {isRtl ? 'اختر موضع التثبيت للشاشة:' : 'Select Screen Docking Preset:'}
              </span>
              <div className="grid grid-cols-2 gap-2">
                {PRESET_POSITIONS.map((pos) => (
                  <button
                    key={pos.id}
                    type="button"
                    onClick={() => handlePresetChange(pos.id)}
                    className={`px-3 py-2 rounded-lg text-start text-[11px] font-medium border transition ${
                      config.preset === pos.id
                        ? 'bg-brand-cyan/20 border-brand-cyan text-cyan-300'
                        : 'bg-slate-900 border-slate-800 text-slate-400 hover:bg-slate-800 hover:text-slate-200'
                    }`}
                  >
                    {isRtl ? pos.labelAr : pos.labelEn}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Section 2: Smart Transparency on Hover */}
        <div className="mb-5 p-3.5 rounded-xl bg-slate-950/70 border border-slate-800 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-purple-400" />
              <div>
                <span className="text-xs font-bold text-slate-200 block">
                  {isRtl ? 'الشفافية بعد فترة التأشير / التوقف' : 'Auto-Transparency on Idle/Hover'}
                </span>
                <span className="text-[10px] text-slate-400 block">
                  {isRtl ? 'يتحول الودجت لشفاف ناعم بعد مدة ثوانٍ محددة' : 'Becomes translucent after configured idle delay'}
                </span>
              </div>
            </div>
            <input
              type="checkbox"
              checked={config.auto_transparency}
              onChange={(e) => setConfig((prev) => ({ ...prev, auto_transparency: e.target.checked }))}
              className="w-4 h-4 accent-brand-cyan cursor-pointer"
            />
          </div>

          {config.auto_transparency && (
            <div className="space-y-3 pt-2.5 border-t border-slate-800/80 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-slate-400 font-medium">
                  {isRtl ? 'مهلة الانتظار قبل التلاشي:' : 'Delay before fade:'}
                </span>
                <div className="flex items-center gap-1.5">
                  {[1, 2, 3, 5].map((sec) => (
                    <button
                      key={sec}
                      type="button"
                      onClick={() => setConfig((prev) => ({ ...prev, transparency_delay_secs: sec }))}
                      className={`px-2.5 py-1 rounded-md text-xs font-mono font-semibold border transition ${
                        config.transparency_delay_secs === sec
                          ? 'bg-purple-600/30 border-purple-500 text-purple-200'
                          : 'bg-slate-900 border-slate-800 text-slate-400 hover:bg-slate-800'
                      }`}
                    >
                      {sec} {isRtl ? 'ث' : 's'}
                    </button>
                  ))}
                </div>
              </div>

              {/* Opacity Range Slider */}
              <div className="pt-2 border-t border-slate-800/50 space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-slate-400 font-medium">
                    {isRtl ? 'مستوى الشفافية عند التلاشي:' : 'Transparency Opacity Level:'}
                  </span>
                  <span className="font-mono font-bold text-brand-cyan">
                    {config.transparency_opacity ?? 30}%
                  </span>
                </div>
                <input
                  type="range"
                  min="10"
                  max="90"
                  step="5"
                  value={config.transparency_opacity ?? 30}
                  onChange={(e) => setConfig((prev) => ({ ...prev, transparency_opacity: Number(e.target.value) }))}
                  className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-brand-cyan"
                />
                <div className="flex justify-between text-[10px] text-slate-500 font-sans">
                  <span>{isRtl ? 'شفاف جداً (10%)' : 'Very Translucent (10%)'}</span>
                  <span>{isRtl ? 'شبه معتم (90%)' : 'Semi-Opaque (90%)'}</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Section 3: Color Themes */}
        <div className="mb-6 space-y-2.5">
          <label className="text-xs font-bold text-slate-300 flex items-center gap-2">
            <Palette className="w-4 h-4 text-emerald-400" />
            <span>{isRtl ? 'نظام ألوان النصوص والأسهم:' : 'Text & Icon Color Theme:'}</span>
          </label>

          <div className="space-y-1.5">
            {COLOR_THEMES.map((theme) => {
              const isSelected = config.color_theme === theme.id;
              return (
                <button
                  key={theme.id}
                  type="button"
                  onClick={() => setConfig((prev) => ({ ...prev, color_theme: theme.id }))}
                  className={`w-full flex items-center justify-between p-2.5 rounded-xl border text-xs font-medium transition ${
                    isSelected
                      ? 'bg-slate-800/90 border-brand-cyan/60 text-white ring-1 ring-brand-cyan/20'
                      : 'bg-slate-950/50 border-slate-800 text-slate-400 hover:bg-slate-900 hover:text-slate-200'
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <div className="flex items-center gap-1 p-1 rounded bg-slate-950 border border-slate-800">
                      <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: theme.previewDown }} />
                      <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: theme.previewUp }} />
                    </div>
                    <span>{isRtl ? theme.nameAr : theme.nameEn}</span>
                  </div>
                  {isSelected && <Check className="w-4 h-4 text-brand-cyan" />}
                </button>
              );
            })}
          </div>
        </div>
        </div>

        {/* Action Buttons (Fixed Footer Bar) */}
        <div className="flex items-center justify-between gap-2.5 p-4 border-t border-slate-800 bg-slate-950/80 shrink-0 backdrop-blur-sm">
          <button
            type="button"
            onClick={async () => {
              try {
                const { invoke } = await import('@tauri-apps/api/core');
                await invoke('reset_widget_position');
              } catch (e) {
                console.error(e);
              }
            }}
            className="px-3.5 py-2 rounded-xl bg-slate-800/80 hover:bg-slate-700 border border-slate-700/80 text-cyan-400 hover:text-cyan-300 transition text-xs font-semibold flex items-center gap-1.5"
            title={isRtl ? 'إعادة الودجت لموضعه الافتراضي فوق شريط المهام' : 'Reset widget to default position above taskbar'}
          >
            <span>📍</span>
            <span>{isRtl ? 'إعادة ضبط الموضع' : 'Reset Position'}</span>
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 font-semibold transition text-xs"
            >
              {isRtl ? 'إلغاء' : 'Cancel'}
            </button>
            <button
              type="button"
              onClick={handleSave}
              disabled={saving}
              className="px-5 py-2 rounded-xl bg-brand-cyan hover:bg-cyan-400 text-slate-950 font-bold transition text-xs shadow-lg shadow-brand-cyan/20"
            >
              {saving ? (isRtl ? 'جاري الحفظ...' : 'Saving...') : isRtl ? 'تطبيق وحفظ' : 'Apply & Save'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
