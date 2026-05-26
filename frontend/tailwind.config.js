/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        display: ['"IBM Plex Sans"', 'ui-sans-serif', 'system-ui', '-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'Roboto', '"Helvetica Neue"', 'Arial', 'sans-serif'],
        mono: ['"IBM Plex Mono"', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
      },
      colors: {
        lg: {
          bg: '#ffffff',
          'bg-deep': '#fafaf8',
          surface: '#f4f4f1',
          'surface-2': '#eeeee9',
          border: '#d8d8d0',
          'border-strong': '#c4c4ba',
          text: '#1a1a1a',
          ink: '#0d0d0d',
          'text-sec': '#5c5c5c',
          'text-muted': '#999990',
          accent: '#1a1a1a',
          'accent-bright': '#1a1a1a',
          'accent-hover': '#333333',
          'accent-soft': 'rgba(26, 26, 26, 0.06)',
          'accent-border': 'rgba(26, 26, 26, 0.18)',
          'code-bg': '#f0f0ec',
          'code-text': '#3d3d3d',
          green: '#2d6a4f',
          red: '#9b2c2c',
          rose: '#9b2c2c',
          blue: '#2b4c8c',
          cyan: '#2b4c8c',
        },
      },
      letterSpacing: {
        tightx: '-0.02em',
      },
    },
  },
  plugins: [],
}
