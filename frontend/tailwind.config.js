/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        display: ['"Space Grotesk"', 'ui-sans-serif', 'system-ui', 'sans-serif'],
      },
      colors: {
        lg: {
          bg: '#000000',
          'bg-deep': '#020204',
          surface: '#0a0a10',
          'surface-2': '#111118',
          border: '#1a1a24',
          'border-strong': '#26263a',
          text: '#ececf1',
          'text-sec': '#8b8b9e',
          'text-muted': '#55556a',
          accent: '#8b5cf6',
          'accent-bright': '#a78bfa',
          'accent-hover': '#c4b5fd',
          'accent-soft': 'rgba(139, 92, 246, 0.12)',
          'accent-border': 'rgba(139, 92, 246, 0.3)',
          cyan: '#06b6d4',
          rose: '#f43f5e',
        },
      },
      letterSpacing: {
        tightx: '-0.02em',
      },
    },
  },
  plugins: [],
}
