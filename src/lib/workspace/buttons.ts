import type { Mapping } from "./client";

/** Matches `SENDABLE` in `remote/mappings.rs`. */
export const sendable: { id: string; label: string }[] = [
  { id: "HOME", label: "Home" },
  { id: "BACK", label: "Back" },
  { id: "MENU", label: "Menu" },
  { id: "VOLUME_UP", label: "Volume up" },
  { id: "VOLUME_DOWN", label: "Volume down" },
  { id: "VOLUME_MUTE", label: "Mute" },
  { id: "MEDIA_PLAY_PAUSE", label: "Play or pause" },
  { id: "MEDIA_NEXT", label: "Next track" },
  { id: "MEDIA_PREVIOUS", label: "Previous track" },
  { id: "MEDIA_STOP", label: "Stop" },
  { id: "MEDIA_REWIND", label: "Rewind" },
  { id: "MEDIA_FAST_FORWARD", label: "Fast forward" },
  { id: "CHANNEL_UP", label: "Channel up" },
  { id: "CHANNEL_DOWN", label: "Channel down" },
  { id: "TV_INPUT", label: "Switch input" },
  { id: "CAPTIONS", label: "Subtitles" },
  { id: "SETTINGS", label: "Settings" },
  { id: "SEARCH", label: "Search" },
  { id: "SLEEP", label: "Turn the screen off" },
];

/** The word the listener logs for a mapping. */
export const logTarget = (mapping: Mapping) =>
  mapping.send ? `KEYCODE_${mapping.send}` : mapping.component;

export const resultText = (mapping: Mapping, failed: boolean) => {
  if (mapping.send)
    return failed
      ? `The TV saw the press but couldn't send ${mapping.label}.`
      : `It worked: the TV sent ${mapping.label}.`;
  return failed
    ? `The TV saw the press but couldn't open ${mapping.label}.`
    : `It worked: ${mapping.label} opened.`;
};
