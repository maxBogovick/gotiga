/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    colors: {

      // ─── Ivory Canvas (backgrounds) ────────────────────────────────────
      'ivory': {
        '50': '#FDFAF5',   // lightest surface, modal bg
        '100': '#FAF6EE',   // page background
        '200': '#F4EBD9',   // cards, panels
        '300': '#EAD9C0',   // deep panels, sidebars
        '400': '#DEC9A8',   // borders, dividers
        '500': '#CEAF86',   // strong borders, decorative lines
      },

      // ─── Ink (text hierarchy) ──────────────────────────────────────────
      'ink': {
        '900': '#180D05',   // near-black — rarely used
        '800': '#2C1710',   // primary text, headings
        '700': '#3E2214',   // secondary headings
        '600': '#5A3420',   // body text
        '500': '#7A5035',   // secondary text
        '400': '#A0745A',   // muted / placeholder
        '300': '#C4A088',   // disabled / metadata
      },

      // ─── Ember (primary accent — terracotta) ───────────────────────────
      'ember': {
        '100': '#FCE8DF',   // tinted bg, badges
        '200': '#F5C5AD',   // hover tints
        '400': '#D97B52',   // mid accent
        '500': '#C0582C',   // primary CTA, focus rings
        '600': '#9A4120',   // pressed, active
        '700': '#722F14',   // dark variant
      },

      // ─── Ochre (golden accent — warmth & richness) ────────────────────
      'ochre': {
        '100': '#FBF0D4',   // light badge bg
        '200': '#F5D98A',   // tag, highlight
        '400': '#D4A832',   // mid gold
        '500': '#B08820',   // primary gold
        '600': '#856615',   // deep gold
      },

      // ─── Sage (nature accent — rare, editorial use) ───────────────────
      'sage': {
        '100': '#EAF0E4',
        '300': '#A8BF96',
        '500': '#6B8A56',
        '700': '#3E5733',
      },

      // ─── Dust (cool warm grey) ─────────────────────────────────────────
      'dust': {
        '100': '#F0E8E0',
        '200': '#D8CDCA',
        '400': '#A89890',
        '600': '#6E5C56',
      },

      // ─── Utility ───────────────────────────────────────────────────────
      'transparent': 'transparent',
      'current': 'currentColor',
      'white': '#FDFAF5',
      'black': '#180D05',
    },

    fontFamily: {
      'display': ['"Fraunces"', 'Georgia', 'serif'],
      'body': ['"DM Sans"', 'system-ui', 'sans-serif'],
      'serif': ['"EB Garamond"', 'Georgia', 'serif'],
      'mono': ['"JetBrains Mono"', 'monospace'],
    },

    fontWeight: {
      'light': '300',
      'normal': '400',
      'medium': '500',
    },

    fontSize: {
      'xs': ['0.6875rem', { lineHeight: '1.6', letterSpacing: '0.02em' }],
      'sm': ['0.8125rem', { lineHeight: '1.65', letterSpacing: '0.01em' }],
      'base': ['1rem', { lineHeight: '1.85' }],
      'lg': ['1.1875rem', { lineHeight: '1.65' }],
      'xl': ['1.4375rem', { lineHeight: '1.45' }],
      '2xl': ['1.75rem', { lineHeight: '1.35' }],
      '3xl': ['2.25rem', { lineHeight: '1.25' }],
      '4xl': ['3rem', { lineHeight: '1.15' }],
      '5xl': ['3.75rem', { lineHeight: '1.08' }],
      '6xl': ['5rem', { lineHeight: '1.0' }],
    },

    letterSpacing: {
      'tighter': '-0.03em',
      'tight': '-0.015em',
      'normal': '0em',
      'wide': '0.04em',
      'wider': '0.08em',
      'widest': '0.14em',
    },

    borderRadius: {
      'none': '0',
      'sm': '3px',
      'DEFAULT': '6px',
      'md': '8px',
      'lg': '12px',
      'xl': '18px',
      '2xl': '24px',
      'full': '9999px',
    },

    extend: {

      // ─── Spacing extras ─────────────────────────────────────────────
      spacing: {
        '18': '4.5rem',
        '22': '5.5rem',
        '26': '6.5rem',
        '30': '7.5rem',
        '34': '8.5rem',
        '88': '22rem',
        '96': '24rem',
        '128': '32rem',
      },

      // ─── Transitions ────────────────────────────────────────────────
      transitionDuration: {
        '350': '350ms',
        '450': '450ms',
        '600': '600ms',
        '800': '800ms',
        '1000': '1000ms',
        '1500': '1500ms',
      },

      transitionTimingFunction: {
        'atelier': 'cubic-bezier(0.22, 0.1, 0.2, 1)',
        'spring': 'cubic-bezier(0.34, 1.56, 0.64, 1)',
        'ease-out-quart': 'cubic-bezier(0.165, 0.84, 0.44, 1)',
      },

      // ─── Shadows (layered, warm-tinted) ─────────────────────────────
      boxShadow: {
        // Elevation system
        'xs': '0 1px 3px rgba(60, 25, 10, 0.06), 0 1px 2px rgba(60, 25, 10, 0.04)',
        'sm': '0 2px 8px rgba(60, 25, 10, 0.07), 0 1px 3px rgba(60, 25, 10, 0.05)',
        'md': '0 4px 16px rgba(60, 25, 10, 0.09), 0 2px 6px rgba(60, 25, 10, 0.06)',
        'lg': '0 8px 32px rgba(60, 25, 10, 0.11), 0 4px 12px rgba(60, 25, 10, 0.07)',
        'xl': '0 16px 56px rgba(60, 25, 10, 0.14), 0 8px 20px rgba(60, 25, 10, 0.08)',
        '2xl': '0 32px 80px rgba(60, 25, 10, 0.18), 0 16px 32px rgba(60, 25, 10, 0.1)',
        // Inset
        'inner-sm': 'inset 0 1px 4px rgba(60, 25, 10, 0.08)',
        'inner-md': 'inset 0 2px 10px rgba(60, 25, 10, 0.1)',
        // Ember glow
        'ember': '0 4px 20px rgba(192, 88, 44, 0.22), 0 2px 8px rgba(192, 88, 44, 0.14)',
        'ember-lg': '0 8px 40px rgba(192, 88, 44, 0.28), 0 4px 16px rgba(192, 88, 44, 0.16)',
        // Highlight for cards
        'card': '0 2px 1px rgba(253, 250, 245, 0.8) inset, 0 8px 32px rgba(60, 25, 10, 0.1)',
      },

      // ─── Animations ─────────────────────────────────────────────────
      animation: {
        'fade-in': 'fadeIn 500ms cubic-bezier(0.22, 0.1, 0.2, 1) both',
        'fade-in-slow': 'fadeIn 900ms cubic-bezier(0.22, 0.1, 0.2, 1) both',
        'slide-up': 'slideUp 600ms cubic-bezier(0.22, 0.1, 0.2, 1) both',
        'slide-up-slow': 'slideUp 900ms cubic-bezier(0.22, 0.1, 0.2, 1) both',
        'scale-in': 'scaleIn 500ms cubic-bezier(0.34, 1.56, 0.64, 1) both',
        'blur-in': 'blurIn 700ms cubic-bezier(0.22, 0.1, 0.2, 1) both',
        'shimmer': 'shimmer 2.4s ease-in-out infinite',
        'float': 'float 6s ease-in-out infinite',
      },

      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(14px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.96)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        blurIn: {
          '0%': { opacity: '0', filter: 'blur(6px)' },
          '100%': { opacity: '1', filter: 'blur(0)' },
        },
        shimmer: {
          '0%, 100%': { opacity: '0.4' },
          '50%': { opacity: '0.8' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0px)' },
          '50%': { transform: 'translateY(-8px)' },
        },
      },

      // ─── Opacity ────────────────────────────────────────────────────
      opacity: {
        '3': '0.03',
        '7': '0.07',
        '12': '0.12',
        '15': '0.15',
        '35': '0.35',
        '65': '0.65',
        '85': '0.85',
        '92': '0.92',
      },

      // ─── Blur ───────────────────────────────────────────────────────
      backdropBlur: {
        'xs': '2px',
        'sm': '6px',
        'md': '12px',
        'lg': '20px',
      },

      // ─── Rotate (organic tilts) ─────────────────────────────────────
      rotate: {
        '0.5': '0.5deg',
        '1': '1deg',
        '2': '2deg',
        '3': '3deg',
        '-0.5': '-0.5deg',
        '-1': '-1deg',
        '-2': '-2deg',
        '-3': '-3deg',
      },

      // ─── Z-index scale ──────────────────────────────────────────────
      zIndex: {
        '1': '1',
        '2': '2',
        '60': '60',
        '70': '70',
        '80': '80',
        '90': '90',
        '100': '100',
      },

      // ─── Aspect ratios ──────────────────────────────────────────────
      aspectRatio: {
        'portrait': '3 / 4',
        'landscape': '4 / 3',
        'product': '5 / 6',
        'wide': '16 / 9',
      },
    },
  },
  plugins: [],
}