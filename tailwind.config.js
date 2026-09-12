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
          bg: '#0B0F17',
          surface: '#121824',
          card: '#182234',
          border: '#223048',
          hover: '#1F2C42'
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
