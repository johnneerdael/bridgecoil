/**
 * Walks the Shell's word tree. Everything here is pure, so the tree can be tested word by word
 * without a TV: what comes next, what the line says, and whether it's complete.
 */
export type Effect = "read" | "change" | "danger";

/** A live list Bridgecoil reads from the TV. See `ShellList` in the Rust query layer. */
export type ListSource =
  | "packages"
  | "disabledPackages"
  | "userPackages"
  | "permissions"
  | "settings"
  | "properties"
  | "services"
  | "cmdServices"
  | "users"
  | "processes"
  | "shellProcesses"
  | "activities"
  | "appOps"
  | "interfaces"
  | "keyboards"
  | "features"
  | "termuxPackages"
  | "termuxFiles";

export type Word = {
  /** What the row shows and, unless `as` says otherwise, what goes on the line. */
  word: string;
  about: string;
  /** The exact text for the command, when it differs from the row's name. */
  as?: string;
  effect?: Effect;
  warning?: string;
  /** Heading the row is listed under. */
  group?: string;
  /** Only offered from this Android SDK. */
  minSdk?: number;
  /** Only offered up to this Android SDK. */
  maxSdk?: number;
  /** The program a first word depends on, when it isn't the word's own name. */
  needs?: string;
  next?: Step | (() => Step);
  /** Everything picked after this word is handed to this function when the line is built. */
  wrap?: (inner: string) => string;
};

/** Shown instead of a dropdown when only the user can know the answer. */
export type Helper = {
  label: string;
  /** What to enter, in plain words. */
  about: string;
  example: string;
  /** Rules out a typo before it reaches the TV. */
  pattern?: RegExp;
  /** How to find the value. One tap runs `command` and shows its result. */
  find?: { label: string; command: string };
  /** Changes the text before it's quoted, such as `input text` needing %s for a space. */
  encode?: (value: string) => string;
};

export type Step = (
  | { kind: "words"; words: Word[] }
  | {
      kind: "list";
      source: ListSource;
      /** Read from an earlier pick, such as the app a permission belongs to. */
      argument?: string;
      /** Rows offered above the live list, with their own next steps. */
      extra?: Word[];
      /** Explains rows the TV only gives a name for. */
      describe?: Record<string, string>;
      other?: Helper;
    }
  | { kind: "path"; folders?: boolean; root?: string; other?: Helper }
  | { kind: "text"; helper: Helper }
) & {
  prompt: string;
  /** The line can run as it stands, and whatever this step offers is optional. */
  optional?: boolean;
  /** Remembers the value for a later list, as `argument`. */
  remember?: string;
  /** The value only feeds a later list and isn't part of the command. */
  silent?: boolean;
  next?: Step | (() => Step);
  /** Applies to lines that end through this step. */
  effect?: Effect;
  warning?: string;
};

export type Pick = {
  /** The text the field shows. */
  text: string;
  /** The text for the command, after quoting and placeholders. */
  command: string;
  word?: Word;
  /** What this pick led to. Null when the line ends here. */
  after: Step | null;
  /** The step this pick answered, for going back. */
  from: Step;
  effect?: Effect;
  warning?: string;
  remember?: [string, string];
};
export type Line = Pick[];

/** Quotes a value for the TV's shell, leaving plain words as they are. */
export function shellQuote(value: string) {
  return /^[\w@%+=:,./-]+$/.test(value)
    ? value
    : `'${value.replaceAll("'", `'"'"'`)}'`;
}

const resolve = (step: Word["next"]) =>
  (typeof step === "function" ? step() : step) ?? null;
const rank: Record<Effect, number> = { read: 0, change: 1, danger: 2 };

/** The strongest effect of anything picked so far. Plain words only read. */
export function effectOf(line: Line): Effect {
  return line.reduce<Effect>(
    (worst, pick) =>
      pick.effect && rank[pick.effect] > rank[worst] ? pick.effect : worst,
    "read",
  );
}
export const warningsOf = (line: Line) => [
  ...new Set(line.flatMap((pick) => (pick.warning ? [pick.warning] : []))),
];

/** Adding to the end of a command that only reads, to narrow down what comes back. */
export const pipes: Step = {
  kind: "words",
  prompt: "Filter the result",
  optional: true,
  words: [
    {
      word: "| grep",
      about: "Keep only the lines containing some text.",
      next: () =>
        filterText("Text to keep", "Only lines containing this are kept."),
    },
    {
      word: "| grep -i",
      about: "The same, ignoring capitals.",
      next: () =>
        filterText("Text to keep", "Only lines containing this are kept."),
    },
    {
      word: "| grep -v",
      about: "Drop the lines containing some text.",
      next: () =>
        filterText("Text to drop", "Lines containing this are left out."),
    },
    {
      word: "| head -n",
      about: "Keep only the first lines.",
      next: () => count("Lines to keep"),
    },
    {
      word: "| tail -n",
      about: "Keep only the last lines.",
      next: () => count("Lines to keep"),
    },
    { word: "| sort", about: "Put the lines in order." },
    { word: "| sort -r", about: "Put the lines in reverse order." },
    { word: "| uniq", about: "Drop a line that repeats the one before it." },
    { word: "| wc -l", about: "Count the lines instead of showing them." },
  ],
};
const filterText = (label: string, about: string): Step => ({
  kind: "text",
  prompt: label,
  helper: { label, about, example: "error" },
});
const count = (label: string): Step => ({
  kind: "text",
  prompt: label,
  helper: {
    label,
    about: "A whole number.",
    example: "20",
    pattern: /^\d{1,6}$/,
  },
});

/** The step after a pick, with the filters offered once a read-only line has nothing left. */
function following(after: Step | null, line: Line): Step | null {
  if (after) return after;
  return effectOf(line) === "read" ? pipes : null;
}

/** What the dropdown should offer next. Null means the line is finished. */
export function stepAfter(line: Line, root: Step): Step | null {
  if (!line.length) return root;
  return following(line[line.length - 1].after, line);
}

/** Whether Run can be pressed. */
export function complete(line: Line, root: Step) {
  if (!line.length) return false;
  const last = line[line.length - 1];
  return !last.after || !!last.after.optional;
}

/** `installed` is only passed for the first word, which is the program itself. */
export const visible = (word: Word, sdk: number, installed?: Set<string>) =>
  (word.minSdk === undefined || sdk >= word.minSdk) &&
  (word.maxSdk === undefined || sdk <= word.maxSdk) &&
  (!installed || installed.has(word.needs ?? word.word.split(" ")[0]));

/** Which step follows a value in this step. */
const onward = (step: Step): Step | null => resolve(step.next);

function fill(text: string, context: Context) {
  return text.replaceAll("{user}", String(context.user));
}
export type Context = { user: number };

export function pickWord(
  line: Line,
  step: Step,
  word: Word,
  context: Context,
): Line {
  const next = resolve(word.next) ?? onward(step);
  const literal = fill(word.as ?? word.word, context);
  return [
    ...line,
    {
      text: word.wrap ? word.word : literal,
      command: literal,
      word,
      from: step,
      after: next,
      effect: word.effect ?? step.effect,
      warning: word.warning ?? step.warning,
    },
  ];
}

export function pickValue(line: Line, step: Step, value: string): Line {
  const helper =
    step.kind === "text"
      ? step.helper
      : "other" in step
        ? step.other
        : undefined;
  const quoted = step.silent
    ? ""
    : shellQuote(helper?.encode ? helper.encode(value) : value);
  return [
    ...line,
    {
      text: quoted,
      command: quoted,
      from: step,
      after: onward(step),
      effect: step.effect,
      warning: step.warning,
      remember: step.remember ? [step.remember, value] : undefined,
    },
  ];
}

/** Values kept for lists further along, such as the app whose permissions are wanted. */
export function remembered(line: Line, name: string) {
  return [...line].reverse().find((pick) => pick.remember?.[0] === name)
    ?.remember?.[1];
}

/** The exact command for the line, and what the field shows for it. */
export function commandOf(line: Line) {
  const wrapAt = line.findIndex((pick) => pick.word?.wrap);
  const words = (picks: Line) =>
    picks
      .map((pick) => pick.command)
      .filter(Boolean)
      .join(" ");
  if (wrapAt < 0) return words(line);
  const inner = words(line.slice(wrapAt + 1));
  return line[wrapAt].word!.wrap!(inner);
}
export const textOf = (line: Line) =>
  line
    .map((pick) => pick.text)
    .filter(Boolean)
    .join(" ");

/** Rows for a word step, narrowed by what's been typed. */
export function matches<T extends { word: string; about: string }>(
  rows: T[],
  search: string,
) {
  const needle = search.trim().toLowerCase();
  if (!needle) return rows;
  return rows.filter(
    (row) =>
      row.word.toLowerCase().includes(needle) ||
      row.about.toLowerCase().includes(needle),
  );
}

const plainToken = /^[\w@%+=:,./-]+$/;
const shellSyntax = /[;&|<>$`\\'"\n()]/;

/**
 * True when a typed command is one the word tree knows only reads. Anything else, including any
 * shell syntax at all, gets a review, so this can only ever save a tap.
 */
export function knownRead(command: string, root: Step, sdk: number): boolean {
  if (!command.trim() || shellSyntax.test(command)) return false;
  const tokens = command.trim().split(/\s+/);
  if (tokens.some((token) => !plainToken.test(token))) return false;
  return walk(root, tokens, 0, sdk);
}

function walk(
  step: Step | null,
  tokens: string[],
  at: number,
  sdk: number,
): boolean {
  if (!step) return at === tokens.length;
  if (at === tokens.length) return !!step.optional;
  if (step.effect && step.effect !== "read") return false;
  const words =
    step.kind === "words"
      ? step.words
      : step.kind === "list"
        ? (step.extra ?? [])
        : [];
  for (const word of words) {
    if (word.effect && word.effect !== "read") continue;
    if (word.wrap || !visible(word, sdk)) continue;
    const literal = (word.as ?? word.word)
      .replace(/^timeout \d+ /, "")
      .split(/\s+/)
      .filter(Boolean);
    if (literal.some((part) => part.includes("{"))) continue;
    if (!literal.every((part, i) => tokens[at + i] === part)) continue;
    const next = resolve(word.next) ?? onward(step);
    if (walk(next, tokens, at + literal.length, sdk)) return true;
  }
  if (step.kind !== "words" && !tokens[at].startsWith("-")) {
    if (walk(onward(step), tokens, at + 1, sdk)) return true;
  }
  return step.optional ? at === tokens.length : false;
}

/** The shell's scrollback, kept while the app runs so leaving Shell doesn't lose it. */
export type Entry = {
  id: number;
  command: string;
  output: string;
  failed: boolean;
  note?: string;
};
const scrollbacks = new Map<string, Entry[]>();
const limit = 1_000_000;
export const scrollback = (device: string) => scrollbacks.get(device) ?? [];
export function keep(device: string, entries: Entry[]) {
  let total = 0;
  const kept: Entry[] = [];
  for (const entry of [...entries].reverse()) {
    total += entry.output.length;
    if (kept.length && total > limit) break;
    kept.unshift(entry);
  }
  scrollbacks.set(device, kept);
  return kept;
}
/** Takes the "$ command" heading off a stored result. */
export function outputOf(stored: string) {
  return stored.startsWith("$ ")
    ? stored.slice(stored.indexOf("\n\n") + 2 || stored.length)
    : stored;
}
