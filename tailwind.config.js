/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    // Полностью переопределяем цвета (НЕ extend!)
    colors: {
      // Основные цвета проекта
      'cabinet': {
        'bg': '#2E2B28',           // тёмный тёплый фон
        'bg-deep': '#1A1816',      // глубокий темный из примера
        'wood': '#5A524C',         // дерево
        'wood-light': 'rgba(90, 82, 76, 0.2)', // translucent wood
        'wood-muted': 'rgba(90, 82, 76, 0.3)', // muted wood for placeholders
        'fabric': '#8C7E73',       // состаренная ткань
        'bone': '#CFC6B8',         // кость / кожа (основной текст)
        'bone-highlight': 'rgba(207, 198, 184, 0.05)', // highlight for zones
        'bone-border': 'rgba(207, 198, 184, 0.1)',    // border for zones
        'dust': '#A39B91',         // пыль (вторичный текст)
        'muted': '#9C8E7D',        // приглушенный акцент из примера
        'glow': '#D4CDB8',         // свечение из примера
      },
      // Акценты (использовать ОЧЕНЬ редко)
      'accent': {
        'red': '#7A2E2E',          // выцветший бордовый
        'olive': '#6A705F',        // пыльно-оливковый
      },
      // Служебные
      'transparent': 'transparent',
      'current': 'currentColor',
      // Чёрный для теней
      'black': '#000000',
    },

    // Шрифты
    fontFamily: {
      'display': ['"Cormorant Garamond"', 'serif'],  // заголовки, имена
      'body': ['"Source Serif 4"', 'serif'],         // основной текст
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
        'cabinet': '0 4px 20px rgba(0, 0, 0, 0.3)',
        'cabinet-lg': '0 8px 40px rgba(0, 0, 0, 0.4)',
        'cabinet-inner': 'inset 0 2px 10px rgba(0, 0, 0, 0.2)',
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
