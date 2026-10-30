import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { afterEach, it, expect } from "vitest";
import App from "./App.svelte";
afterEach(cleanup);
it("browser preview is honest about native features", () => {
  render(App);
  expect(screen.getByText("Native app required")).toBeTruthy();
});
it("cannot falsely pair or install from a browser preview", async () => {
  render(App);
  await fireEvent.input(screen.getByLabelText(/Pairing address/), {
    target: { value: "tv.local:37123" },
  });
  await fireEvent.input(screen.getByLabelText("Six-digit pairing code"), {
    target: { value: "123456" },
  });
  expect(
    (screen.getByRole("button", { name: "Pair TV" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});
