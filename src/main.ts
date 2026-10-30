import { mount } from "svelte";
import App from "./App.svelte";
import "./styles.css";
import { applyTheme, parseTheme, themeKey } from "./lib/theme/theme";
applyTheme(
  parseTheme(localStorage.getItem(themeKey)),
  window.matchMedia("(prefers-color-scheme: dark)").matches,
);
mount(App, { target: document.getElementById("app")! });
