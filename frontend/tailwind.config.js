/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: '#202124',
        coral: '#ff6f61',
        teal: '#13b8a6',
        sunny: '#ffd166',
        lilac: '#9b8cff',
        mint: '#d9fbe8',
        paper: '#fffaf0',
      },
      boxShadow: {
        sticker: '6px 6px 0 #202124',
      },
      fontFamily: {
        sans: [
          'Inter',
          'ui-sans-serif',
          'system-ui',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'sans-serif',
        ],
      },
    },
  },
  plugins: [],
};
