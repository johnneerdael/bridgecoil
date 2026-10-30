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
