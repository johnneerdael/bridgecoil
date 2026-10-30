export type ThemePreference = "system" | "light" | "dark";
export const themeKey = "bridgecoil.theme";

export function parseTheme(value: string | null): ThemePreference {
  return value === "light" || value === "dark" ? value : "system";
}

export function resolveTheme(preference: ThemePreference, systemDark: boolean) {
  return preference === "system" ? (systemDark ? "dark" : "light") : preference;
}

export function applyTheme(preference: ThemePreference, systemDark: boolean) {
  const effective = resolveTheme(preference, systemDark);
  document.documentElement.dataset.theme = effective;
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", effective === "dark" ? "#101318" : "#F1F4F8");
  return effective;
}
