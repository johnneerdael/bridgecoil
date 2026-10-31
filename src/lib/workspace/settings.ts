const scales = ["0", "0.5", "1", "1.5", "2", "5", "10"];
const volumes = ["0", "3", "5", "8", "10", "15"];
const brightness = ["0", "32", "64", "128", "192", "255"];

/** Android reports some values as "1.0" and unset keys as "null", so match by number and keep the rest visible. */
export function settingChoices(key: string, value: string | null) {
  const options = key.endsWith("_scale")
    ? scales
    : key === "volume_music"
      ? volumes
      : brightness;
  const current = value ?? "";
  const selected =
    (current && options.find((o) => Number(o) === Number(current))) || current;
  return {
    selected,
    options: options.includes(selected) ? options : [selected, ...options],
  };
}

export const settingLabel = (value: string | null) =>
  !value ? "Unknown" : value === "null" ? "Not set" : value;

/** A placeholder for a value the TV reported, never something to write back. */
export const settingWritable = (option: string) =>
  option !== "" && option !== "null";
