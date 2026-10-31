import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { afterEach, beforeAll, it, expect } from "vitest";
import App from "./App.svelte";
afterEach(cleanup);
it("keeps input focus during the pointer press on Pair and Connect", async () => {
  render(App);
  for (const [step, inputId, buttonName] of [
    ["1 Pair TV", "pair-code", "Pair TV"],
    ["2 Connect", "connect-address", "Connect to TV"],
  ]) {
    await fireEvent.click(screen.getByRole("button", { name: step }));
    const input = document.getElementById(inputId)!;
    input.focus();
    const button = screen.getByRole("button", {
      name: buttonName,
    }) as HTMLButtonElement;
    button.disabled = false;
    const press = new MouseEvent("pointerdown", {
      bubbles: true,
      cancelable: true,
      button: 0,
    });
    button.dispatchEvent(press);
    expect(press.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(input);
  }
});
it("accepts exactly six digits in the pairing form", async () => {
  render(App);
  const input = screen.getByLabelText(
    "Six-digit pairing code",
  ) as HTMLInputElement;
  for (const value of ["123456", "012345"]) {
    await fireEvent.input(input, { target: { value } });
    expect(input.checkValidity()).toBe(true);
  }
  for (const value of ["12345", "12345a"]) {
    await fireEvent.input(input, { target: { value } });
    expect(input.checkValidity()).toBe(false);
  }
});
it("browser preview is honest about native features", () => {
  render(App);
  expect(
    screen.getByRole("button", { name: /Native app required/ }),
  ).toBeTruthy();
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

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
it("uses the header badge for connection state instead of an early-build badge", () => {
  render(App);
  expect(screen.queryByText("Early build")).toBeNull();
  expect(
    screen.getByRole("button", { name: "Connection status: Not connected" }),
  ).toBeTruthy();
  expect(document.querySelector(".devicebar")).toBeNull();
  expect(
    screen.getByRole("heading", { level: 1, name: "Connect a device" }),
  ).toBeTruthy();
});
it("opens debugging help in a dismissible dialog without expanding the page", async () => {
  render(App);
  await fireEvent.click(
    screen.getByRole("button", { name: "Finding the debugging options" }),
  );
  const dialog = await screen.findByRole("dialog", {
    name: "Finding the debugging options",
  });
  expect(dialog.textContent?.replace(/\s+/g, " ")).toContain(
    "Android TV OS build",
  );
  await fireEvent.click(screen.getByRole("button", { name: "Close dialog" }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("uses a dismissible hamburger menu instead of permanent bottom tabs", async () => {
  render(App);
  await fireEvent.click(
    screen.getByRole("button", { name: "Open navigation" }),
  );
  expect(
    await screen.findByRole("dialog", { name: "Navigation" }),
  ).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Install app" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Connect device" })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: /^Apps/ }));
  expect(screen.getByRole("heading", { level: 1, name: "Apps" })).toBeTruthy();
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.querySelector(".bottom-nav")).toBeNull();
});
