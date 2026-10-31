import { mount } from "svelte";
import Harness from "./Harness.svelte";
import "../../src/styles.css";
document.documentElement.dataset.theme = "light";
mount(Harness, { target: document.getElementById("app")! });
