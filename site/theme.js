// Run before the page is painted so a saved light theme does not flash dark.
(() => {
  let choice = "dark";
  try { choice = localStorage.getItem("bridgecoil-theme") || "dark"; } catch { /* Storage may be blocked. */ }
  if (!["dark", "light", "system"].includes(choice)) choice = "dark";
  const system = window.matchMedia("(prefers-color-scheme: light)");
  const apply = () => {
    const theme = choice === "system" ? (system.matches ? "light" : "dark") : choice;
    document.documentElement.dataset.theme = theme;
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", theme === "dark" ? "#101318" : "#F1F4F8");
  };
  apply();
  system.addEventListener("change", apply);
  document.addEventListener("DOMContentLoaded", () => {
    const select = document.querySelector("#theme");
    if (!select) return;
    select.value = choice;
    select.addEventListener("change", () => {
      choice = select.value;
      try { localStorage.setItem("bridgecoil-theme", choice); } catch { /* Keep working for this visit. */ }
      apply();
    });
  });
})();
