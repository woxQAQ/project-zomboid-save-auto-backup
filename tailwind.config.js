/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Project Zomboid inspired dark theme colors
        background: '#1a1a1a',
        foreground: '#e0e0e0',
        primary: '#d32f2f',
        success: '#388e3c',
        warning: '#fbc02d',
      },
    },
  },
  plugins: [],
}
