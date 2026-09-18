/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        dark: {
          bg: 'var(--bg-app)',
          surface: 'var(--bg-surface)',
          card: 'var(--bg-card)',
          border: 'var(--border-app)',
          hover: 'var(--bg-card-hover)'
        },
        theme: {
          bg: 'var(--bg-app)',
          surface: 'var(--bg-surface)',
          card: 'var(--bg-card)',
          border: 'var(--border-app)',
          hover: 'var(--bg-card-hover)',
          text: 'var(--text-main)',
          muted: 'var(--text-muted)'
        },
        brand: {
          cyan: '#00F0FF',
          emerald: '#10B981',
          rose: '#F43F5E',
          amber: '#F59E0B',
          purple: '#8B5CF6'
        }
      },
      fontFamily: {
        arabic: ['"Segoe UI"', '"Tajawal"', '"Cairo"', 'system-ui', 'sans-serif'],
        sans: ['"Inter"', '"Segoe UI"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"Fira Code"', 'monospace']
      }
    },
  },
  plugins: [],
}
