import { it, expect, vi, afterEach } from "vitest";
import { startNative } from "./native-startup";
afterEach(() => vi.useRealTimers());
const blank =
  'event.listen not allowed on window "main", webview "main", URL: about:blank';
it("waits through the blank-origin startup race before enabling native work", async () => {
  vi.useFakeTimers();
  const status = vi.fn().mockRejectedValueOnce(blank).mockResolvedValue(null);
  const unlisten = vi.fn();
  const listen = vi
    .fn()
    .mockRejectedValueOnce(blank)
    .mockResolvedValue(unlisten);
  const pending = startNative(status, listen, new AbortController().signal);
  await vi.runAllTimersAsync();
  const result = await pending;
  expect(result.status).toBeNull();
  expect(result.unlisten).toBe(unlisten);
  expect(status).toHaveBeenCalledTimes(3);
  expect(listen).toHaveBeenCalledTimes(2);
});
it("does not retry actual permission failures", async () => {
  const status = vi.fn().mockRejectedValue("Local network access denied");
  const listen = vi.fn();
  await expect(
    startNative(status, listen, new AbortController().signal),
  ).rejects.toBe("Local network access denied");
  expect(status).toHaveBeenCalledTimes(1);
  expect(listen).not.toHaveBeenCalled();
});
it("bounds startup retries and leaves the original rejection observable", async () => {
  vi.useFakeTimers();
  const status = vi.fn().mockRejectedValue(blank);
  const pending = startNative(status, vi.fn(), new AbortController().signal);
  const assertion = expect(pending).rejects.toBe(blank);
  await vi.runAllTimersAsync();
  await assertion;
  expect(status.mock.calls.length).toBeLessThanOrEqual(8);
});
it("disposes a listener if the component closes during registration", async () => {
  const controller = new AbortController(),
    unlisten = vi.fn();
  const result = startNative(
    async () => null,
    async () => {
      controller.abort();
      return unlisten;
    },
    controller.signal,
  );
  await expect(result).rejects.toMatchObject({ name: "AbortError" });
  expect(unlisten).toHaveBeenCalledTimes(1);
});
