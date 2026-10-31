const destinations = {
  device: ["Device", "Connection, device identity, active profile and capabilities, together in one place."],
  apps: ["Apps", "Browse packages, review community ratings and inspect the target before changing an app."],
  remote: ["Remote", "Everyday TV controls, app shortcuts and physical-button learning with device-specific checks."],
  tools: ["Tools", "Files, captures, logs, launcher settings and explained commands when you need to go deeper."],
  history: ["History", "A record of what was requested, what was verified and which recovery actions are available."],
};
const tabs = [...document.querySelectorAll("[data-view]")];
const panel = document.querySelector("#workspace-panel");
if (tabs.length && panel) {
  const activate = (tab) => {
    tabs.forEach((item) => {
      const selected = item === tab;
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
    });
    const [name, description] = destinations[tab.dataset.view];
    document.querySelector("#preview-name").textContent = name;
    document.querySelector("#preview-description").textContent = description;
    panel.setAttribute("aria-labelledby", tab.id);
  };
  document.querySelector(".workspace-tabs").hidden = false;
  activate(tabs[0]);
  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => activate(tab));
    tab.addEventListener("keydown", (event) => {
      const next = { ArrowRight: (index + 1) % tabs.length, ArrowLeft: (index + tabs.length - 1) % tabs.length, Home: 0, End: tabs.length - 1 }[event.key];
      if (next === undefined) return;
      event.preventDefault();
      activate(tabs[next]);
      tabs[next].focus();
    });
  });
}
