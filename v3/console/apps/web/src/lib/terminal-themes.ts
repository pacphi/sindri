import type { TerminalTheme } from "@/types/terminal";

export const darkTheme: TerminalTheme = {
  background: "#0d1117",
  foreground: "#e6edf3",
  cursor: "#e6edf3",
  cursorAccent: "#0d1117",
  selectionBackground: "#264f78",
  selectionForeground: "#e6edf3",
  black: "#21262d",
  red: "#ff7b72",
  green: "#3fb950",
  yellow: "#d29922",
  blue: "#58a6ff",
  magenta: "#bc8cff",
  cyan: "#39c5cf",
  white: "#b1bac4",
  brightBlack: "#6e7681",
  brightRed: "#ffa198",
  brightGreen: "#56d364",
  brightYellow: "#e3b341",
  brightBlue: "#79c0ff",
  brightMagenta: "#d2a8ff",
  brightCyan: "#56d4dd",
  brightWhite: "#f0f6fc",
};

export const lightTheme: TerminalTheme = {
  background: "#ffffff",
  foreground: "#1f2328",
  cursor: "#1f2328",
  cursorAccent: "#ffffff",
  selectionBackground: "#add6ff",
  selectionForeground: "#1f2328",
  black: "#f6f8fa",
  red: "#cf222e",
  green: "#116329",
  yellow: "#4d2d00",
  blue: "#0550ae",
  magenta: "#8250df",
  cyan: "#1b7c83",
  white: "#6e7781",
  brightBlack: "#57606a",
  brightRed: "#a40e26",
  brightGreen: "#1a7f37",
  brightYellow: "#633c01",
  brightBlue: "#0969da",
  brightMagenta: "#6639ba",
  brightCyan: "#1b7c83",
  brightWhite: "#24292f",
};

export const themes = {
  dark: darkTheme,
  light: lightTheme,
} as const;

export type ThemeName = keyof typeof themes;
