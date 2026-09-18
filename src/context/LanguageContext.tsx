import React, { createContext, useContext, useState, useEffect } from 'react';
import { en } from '../locales/en';
import { ar } from '../locales/ar';
import { Language } from '../types';

type LocaleDict = typeof en;

interface LanguageContextType {
  lang: Language;
  t: LocaleDict;
  isRtl: boolean;
  setLang: (l: Language) => void;
  toggleLang: () => void;
}

const LanguageContext = createContext<LanguageContextType | null>(null);

export const LanguageProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [lang, setLangState] = useState<Language>(() => {
    const saved = localStorage.getItem('flux_lang') || localStorage.getItem('netflow_lang');
    return (saved === 'ar' || saved === 'en') ? saved : 'ar'; // Default to Arabic as requested
  });

  const setLang = (l: Language) => {
    setLangState(l);
    localStorage.setItem('flux_lang', l);
  };

  const toggleLang = () => {
    setLang(lang === 'ar' ? 'en' : 'ar');
  };

  const isRtl = lang === 'ar';
  const t = isRtl ? ar : en;

  useEffect(() => {
    document.documentElement.lang = lang;
    document.documentElement.dir = isRtl ? 'rtl' : 'ltr';
    if (isRtl) {
      document.body.classList.add('font-arabic');
      document.body.classList.remove('font-sans');
    } else {
      document.body.classList.add('font-sans');
      document.body.classList.remove('font-arabic');
    }
  }, [lang, isRtl]);

  return (
    <LanguageContext.Provider value={{ lang, t, isRtl, setLang, toggleLang }}>
      {children}
    </LanguageContext.Provider>
  );
};

export const useLanguage = () => {
  const ctx = useContext(LanguageContext);
  if (!ctx) throw new Error('useLanguage must be used within LanguageProvider');
  return ctx;
};
