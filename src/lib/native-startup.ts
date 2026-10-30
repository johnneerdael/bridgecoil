/** Retry only the read-only startup handshake when Android still reports its initial blank URL. */
export async function startNative<T>(
  readStatus: () => Promise<T>,
  subscribe: () => Promise<() => void>,
  signal: AbortSignal,
): Promise<{ status: T; unlisten: () => void }> {
  const delays = [25, 50, 100, 200, 400, 800, 800];
  for (let attempt = 0; ; attempt++) {
    if (signal.aborted)
      throw new DOMException("Startup cancelled", "AbortError");
    try {
      const status = await readStatus();
      if (signal.aborted)
        throw new DOMException("Startup cancelled", "AbortError");
      const unlisten = await subscribe();
      if (signal.aborted) {
        unlisten();
        throw new DOMException("Startup cancelled", "AbortError");
      }
      return { status, unlisten };
    } catch (error) {
      const message =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "";
      if (
        signal.aborted ||
        !message.includes("about:blank") ||
        !message.includes("not allowed") ||
        attempt >= delays.length
      )
        throw error;
      // Android's document script may run before onPageStarted updates Wry's IPC URL.
      console.debug(
        "Bridgecoil: waiting for native page initialization",
        attempt + 1,
      );
      await new Promise<void>((resolve) => {
        const done = () => {
          clearTimeout(timer);
          signal.removeEventListener("abort", done);
          resolve();
        };
        const timer = setTimeout(done, delays[attempt]);
        signal.addEventListener("abort", done, { once: true });
        if (signal.aborted) done();
      });
    }
  }
}
