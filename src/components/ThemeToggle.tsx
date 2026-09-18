import React from 'react';
import { Sun, Moon, Monitor } from 'lucide-react';
import { useTheme, ThemeMode } from '../context/ThemeContext';
import { useLanguage } from '../context/LanguageContext';

export const ThemeToggle: React.FC = () => {
  const { themeMode, setThemeMode } = useTheme();
  const { isRtl } = useLanguage();

  const options: { mode: ThemeMode; labelAr: string; labelEn: string; icon: React.ReactNode }[] = [
    {
      mode: 'light',
      labelAr: 'وضع النهار (فاتح)',
      labelEn: 'Light Mode',
      icon: <Sun className="w-3.5 h-3.5 text-amber-500" />,
    },
    {
      mode: 'dark',
      labelAr: 'وضع الليل (داكن)',
      labelEn: 'Dark Mode',
      icon: <Moon className="w-3.5 h-3.5 text-indigo-400" />,
    },
    {
      mode: 'system',
      labelAr: 'تلقائي حسب ثيم النظام',
      labelEn: 'Sync with System',
      icon: <Monitor className="w-3.5 h-3.5 text-brand-cyan" />,
    },
  ];

  return (
    <div
      className="inline-flex items-center p-0.5 rounded-lg bg-slate-200/80 dark:bg-dark-card border border-transparent dark:border-dark-border shadow-sm select-none"
      role="group"
      aria-label="Theme mode switcher"
    >
      {options.map((opt) => {
        const isActive = themeMode === opt.mode;
        return (
          <button
            key={opt.mode}
            onClick={() => setThemeMode(opt.mode)}
            className={`flex items-center justify-center p-1.5 rounded-md transition-all duration-200 cursor-pointer ${
              isActive
                ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm scale-105'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-slate-300/40 dark:hover:bg-slate-800/40'
            }`}
            title={isRtl ? opt.labelAr : opt.labelEn}
          >
            {opt.icon}
          </button>
        );
      })}
    </div>
  );
};
