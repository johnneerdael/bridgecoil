/** How long Quick connect keeps trying after a wake, since a TV needs a moment to come up. */
export const wakeTiming = { retries: 3, pause: 4000 };
/** Only failures a TV that's still waking could cause. A refused key or a wrong TV isn't one. */
export const wakingFailure = (error: unknown) =>
  !!error &&
  typeof error === "object" &&
  "kind" in error &&
  [
    "connection",
    "connectionRefused",
    "connectionClosed",
    "timeout",
    "discovery",
  ].includes(String(error.kind));
