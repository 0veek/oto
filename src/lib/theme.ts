import type { ThemePreset } from "./types";

export function applyTheme(theme: ThemePreset, reduceMotion = false, fontScale = 1) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.reduceMotion = String(reduceMotion);
  document.documentElement.style.fontSize = `${Math.max(0.85, Math.min(1.25, fontScale)) * 100}%`;
}
