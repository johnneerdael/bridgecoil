/** Bound both wrapped width and line count, retaining every character for export/copy. */
export function outputPages(text: string, columns = 36, lines = 10): string[] {
  const pages: string[] = [];
  let page = "",
    row = 0,
    column = 0;
  for (const char of text) {
    const width = char === "\t" ? 4 : char.codePointAt(0)! > 255 ? 2 : 1;
    if (char !== "\n" && column + width > columns) {
      row++;
      column = 0;
    }
    if (row >= lines) {
      pages.push(page);
      page = "";
      row = 0;
      column = 0;
    }
    page += char;
    if (char === "\n") {
      row++;
      column = 0;
    } else column += width;
  }
  if (page || !pages.length) pages.push(page);
  return pages;
}

type BulkOutcome = {
  package: string;
  status: string;
  message?: string | null;
};
const bulkWords: Record<string, string> = {
  verified: "Done",
  sent: "Sent",
  cancelled: "Not run",
  failed: "Failed",
  partial: "Partly done",
  unknown: "Unconfirmed",
};
/** Bulk results are stored as JSON. Show people the outcome, one app to a line. */
export function resultText(kind: string, output: string) {
  if (kind !== "bulkPackages") return output;
  try {
    const result: {
      items: BulkOutcome[];
      excluded?: { package: string; reason: string }[];
    } = JSON.parse(output);
    const line = (word: string, name: string, note?: string | null) =>
      [`${word}: ${name}`, note].filter(Boolean).join(". ");
    return [
      ...result.items.map((item) =>
        line(bulkWords[item.status] ?? item.status, item.package, item.message),
      ),
      ...(result.excluded ?? []).map((item) =>
        line("Left out", item.package, item.reason),
      ),
    ].join("\n");
  } catch {
    return output;
  }
}
