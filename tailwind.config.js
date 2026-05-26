/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    // Полностью переопределяем цвета (НЕ extend!)
    colors: {
      // Основные цвета проекта
      'cabinet': {
        'bg': '#F8F1E7',
        'bg-deep': '#EBDCC8',
        'wood': '#C9A875',
        'wood-light': 'rgba(198, 95, 60, 0.12)',
        'wood-muted': 'rgba(52, 37, 28, 0.78)',
        'fabric': '#A86124',
        'bone': '#34251C',
        'bone-highlight': 'rgba(198, 95, 60, 0.08)',
        'bone-border': 'rgba(198, 95, 60, 0.18)',
        'dust': '#5F4636',
        'muted': '#7C6554',
        'glow': '#C65F3C',
      },
      // Акценты (использовать ОЧЕНЬ редко)
      'accent': {
        'red': '#A94438',
        'olive': '#6F7D45',
      },
      // Служебные
      'transparent': 'transparent',
      'current': 'currentColor',
      // Чёрный для теней
      'black': '#2F2117',
    },

    // Шрифты
    fontFamily: {
      'display': ['"Fraunces"', 'Georgia', 'serif'],
      'body': ['"Inter"', 'system-ui', 'sans-serif'],
    },

    // ВАЖНО: Убираем все font-weight кроме normal
    fontWeight: {
      'normal': '400',
    },

    // Размеры текста
    fontSize: {
      'xs': ['0.75rem', { lineHeight: '1.5' }],
      'sm': ['0.875rem', { lineHeight: '1.6' }],
      'base': ['1rem', { lineHeight: '1.8' }],
      'lg': ['1.25rem', { lineHeight: '1.6' }],
      'xl': ['1.5rem', { lineHeight: '1.4' }],
      '2xl': ['2rem', { lineHeight: '1.3' }],
      '3xl': ['2.5rem', { lineHeight: '1.2' }],
      '4xl': ['3rem', { lineHeight: '1.1' }],
    },

    // Расширяем стандартные значения
    extend: {
      // Анимации (всё медленное!)
      transitionDuration: {
        '400': '400ms',
        '600': '600ms',
        '800': '800ms',
        '1000': '1000ms',
      },

      // Timing functions
      transitionTimingFunction: {
        'cabinet': 'cubic-bezier(0.25, 0.1, 0.25, 1)',
      },

      // Тени (мягкие, тёплые)
      boxShadow: {
        'cabinet': '0 4px 20px rgba(111, 59, 36, 0.12)',
        'cabinet-lg': '0 8px 40px rgba(111, 59, 36, 0.16)',
        'cabinet-inner': 'inset 0 2px 10px rgba(111, 59, 36, 0.12)',
      },

      // Прозрачности для overlay
      opacity: {
        '15': '0.15',
        '85': '0.85',
      },

      // Размытие для эффектов
      backdropBlur: {
        'xs': '2px',
      },

      // Анимации
      animation: {
        'fade-in': 'fadeIn 500ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'fade-in-slow': 'fadeIn 800ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'scale-in': 'scaleIn 500ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
        'slide-up': 'slideUp 600ms cubic-bezier(0.25, 0.1, 0.25, 1) forwards',
      },

      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.98)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },

      // Поворот для "разбросанных" элементов
      rotate: {
        '1': '1deg',
        '2': '2deg',
        '3': '3deg',
        '-1': '-1deg',
        '-2': '-2deg',
        '-3': '-3deg',
      },
    },
  },
  plugins: [],
}
