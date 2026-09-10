/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{vue,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        // 象棋阵营色语义（唯一权威，见 src/assets/main.css 的 --side-*）
        "side-red": "var(--side-red)",
        "side-black": "var(--side-black)",
        "side-red-fg": "var(--side-red-fg)",
        "side-black-fg": "var(--side-black-fg)",
        // 棋盘提示色（随主题在 --board-hint 切换）
        "board-hint": "var(--board-hint)",
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      // 字号梯度语义栈：caption 10 · body-sm 11 · body 12 · title 14
      fontSize: {
        caption: ["0.625rem", { lineHeight: "1.1" }],
        "body-sm": ["0.6875rem", { lineHeight: "1.25" }],
        body: ["0.75rem", { lineHeight: "1.35" }],
        title: ["0.875rem", { lineHeight: "1.4" }],
      },
    },
  },
  plugins: [],
};
