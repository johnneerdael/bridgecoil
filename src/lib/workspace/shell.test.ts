import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeAll, describe, it, expect, vi } from "vitest";
import ShellTool from "./ShellTool.svelte";
import {
  commandOf,
  complete,
  effectOf,
  knownRead,
  keep,
  outputOf,
  pickValue,
  pickWord,
  remembered,
  stepAfter,
  scrollback,
  textOf,
  type Line,
  type Step,
  type Word,
} from "./shell-line";
import { keycodes, root } from "./shell-words";
import type { Platform, Query } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});

const context = { user: 0 };
const resolve = (next: Word["next"]) =>
  typeof next === "function" ? next() : next;
/** Options can come in any order, so only one of each is followed: the rest repeat the same words. */
function steps(step: Step, seen = new Set<Step>(), options = 0): Step[] {
  if (seen.has(step) || options > 1) return [];
  seen.add(step);
  const found = [step];
  const follow = (next: Word["next"]) => {
    const after = resolve(next);
    if (after)
      found.push(
        ...steps(after, seen, options + (typeof next === "function" ? 1 : 0)),
      );
  };
  follow(step.next);
  const words =
    step.kind === "words"
      ? step.words
      : step.kind === "list"
        ? (step.extra ?? [])
        : [];
  for (const word of words) follow(word.next);
  return found;
}
const all = steps(root);
const allWords = all.flatMap((step) =>
  step.kind === "words"
    ? step.words
    : step.kind === "list"
      ? (step.extra ?? [])
      : [],
);
function say(...words: string[]): Line {
  let line: Line = [];
  for (const word of words) {
    const step = stepAfter(line, root)!;
    const options =
      step.kind === "words"
        ? step.words
        : step.kind === "list"
          ? (step.extra ?? [])
          : [];
    const match = options.find((option) => option.word === word);
    line = match
      ? pickWord(line, step, match, context)
      : pickValue(line, step, word);
  }
  return line;
}

describe("the word tree", () => {
  it("offers more than the sources it was drawn from", () => {
    expect(allWords.length).toBeGreaterThan(700);
  });
  it("explains every word and every step", () => {
    for (const word of allWords) expect(word.about, word.word).toBeTruthy();
    for (const step of all) expect(step.prompt).toBeTruthy();
  });
  it("gives every value someone has to type an explanation and an example", () => {
    for (const step of all) {
      const helper =
        step.kind === "text"
          ? step.helper
          : "other" in step
            ? step.other
            : undefined;
      if (step.kind === "text") expect(helper, step.prompt).toBeTruthy();
      if (helper) {
        expect(helper.about, step.prompt).toBeTruthy();
        expect(helper.example, step.prompt).toBeTruthy();
        if (helper.pattern)
          expect(helper.pattern.test(helper.example), step.prompt).toBe(true);
      }
    }
  });
  it("warns about everything marked dangerous", () => {
    for (const word of allWords.filter((w) => w.effect === "danger"))
      expect(word.warning, word.word).toBeTruthy();
  });
  it("never marks something that changes the TV as only reading", () => {
    const changing =
      /^(rm|rmdir|mv|cp|mkdir|touch|chmod|kill|reboot|setprop|screencap|screenrecord|put|delete|disable|enable|clear|uninstall|grant|revoke|suspend|force-stop|keyevent|tap|swipe|text)\b/;
    for (const word of allWords) {
      if (changing.test(word.word) && !word.as?.startsWith("-"))
        expect(word.effect ?? "read", word.word).not.toBe("read");
    }
  });
  it("only builds commands that finish on their own", () => {
    expect(complete(say("logcat"), root)).toBe(false);
    for (const word of allWords) {
      const text = word.as ?? word.word;
      if (!word.next)
        expect(text, "interactive").not.toMatch(/^(vi|nano|sh|bash|sqlite3)$/);
      if (text.startsWith("ping ")) expect(text).toContain("-c");
      if (text.startsWith("top ")) expect(text).toContain("-n 1");
      if (text.includes("getevent -l")) expect(text).toContain("timeout");
    }
  });
  it("covers the TV buttons and every letter", () => {
    const names = keycodes.map((key) => key.word);
    expect(new Set(names).size).toBe(names.length);
    for (const key of [
      "KEYCODE_HOME",
      "KEYCODE_TV_INPUT_HDMI_1",
      "KEYCODE_PROG_RED",
      "KEYCODE_Z",
      "KEYCODE_9",
    ])
      expect(names).toContain(key);
  });
  it("only lists apps you added for uninstall", () => {
    const line = say("pm", "uninstall", "No more options");
    expect(stepAfter(line, root)).toMatchObject({
      kind: "list",
      source: "userPackages",
    });
  });
  it("has no words that only work from a computer", () => {
    for (const word of allWords)
      expect(word.as ?? word.word, word.word).not.toMatch(/^(adb|fastboot) /);
  });
});

describe("building a line", () => {
  it("moves through the words, then lists what's on the TV", () => {
    const line = say("pm", "grant");
    expect(stepAfter(line, root)).toMatchObject({
      kind: "list",
      source: "packages",
    });
    const app = say("pm", "grant", "org.demo");
    expect(remembered(app, "package")).toBe("org.demo");
    expect(stepAfter(app, root)).toMatchObject({
      kind: "list",
      source: "permissions",
      argument: "package",
    });
    expect(complete(app, root)).toBe(false);
    const done = say("pm", "grant", "org.demo", "android.permission.CAMERA");
    expect(complete(done, root)).toBe(true);
    expect(commandOf(done)).toBe("pm grant org.demo android.permission.CAMERA");
    expect(effectOf(done)).toBe("change");
  });
  it("fills in the connected profile", () => {
    const step = stepAfter(say("pm"), root)!;
    const word = (step as Extract<Step, { kind: "words" }>).words.find(
      (w) => w.word === "enable",
    )!;
    const line = pickWord(say("pm"), step, word, { user: 10 });
    expect(commandOf(line)).toBe("pm enable --user 10");
  });
  it("quotes whatever someone types, so it can't become another command", () => {
    const line = say("logcat", "-d -t 100", "-s", "x; reboot");
    expect(commandOf(line)).toBe("logcat -d -t 100 -s 'x; reboot'");
    expect(effectOf(line)).toBe("read");
  });
  it("offers filters after a command that only reads", () => {
    const line = say("uptime");
    expect(complete(line, root)).toBe(true);
    expect(textOf(say("uptime", "| wc -l"))).toBe("uptime | wc -l");
  });
  it("offers no filters after a change", () => {
    expect(stepAfter(say("sync"), root)).toBeNull();
  });
  it("types spaces the way the TV's text input needs them", () => {
    expect(commandOf(say("input", "text", "hello there"))).toBe(
      "input text hello%sthere",
    );
  });
  it("runs Termux words inside Termux, and shows them as Termux words", () => {
    const line = say("termux", "termux-info");
    expect(textOf(line)).toBe("termux termux-info");
    expect(commandOf(line)).toContain("run-as com.termux");
    expect(commandOf(line)).toContain("PREFIX=");
    expect(commandOf(line)).toMatch(/bash -c 'termux-info'$/);
  });
  it("runs a Termux install in the background so it can outlast the time limit", () => {
    const command = commandOf(say("termux", "pkg install", "python"));
    expect(command).toContain("nohup");
    expect(command).toContain("apt-get -y install python");
    expect(effectOf(say("termux", "pkg install", "python"))).toBe("change");
  });
  it("uses a value only for a later list when the command doesn't need it", () => {
    const line = say("am", "start", "-n", "org.demo", "org.demo/.Main");
    expect(commandOf(line)).toBe("am start -n org.demo/.Main");
  });
  it("asks for an option's value, then carries on with the other options", () => {
    const line = say("logcat", "-d -t 100", "-s", "ActivityManager");
    expect(complete(say("logcat", "-d -t 100", "-s"), root)).toBe(false);
    expect(complete(line, root)).toBe(true);
    expect(
      commandOf(
        say("logcat", "-d -t 100", "-s", "ActivityManager", "-b crash"),
      ),
    ).toBe("logcat -d -t 100 -s ActivityManager -b crash");
  });
  it("lets a step be left out", () => {
    expect(complete(say("getprop"), root)).toBe(true);
    expect(commandOf(say("getprop", "ro.product.model"))).toBe(
      "getprop ro.product.model",
    );
  });
});

describe("what a typed command needs", () => {
  const reads = (command: string) => knownRead(command, root, 34);
  it("runs what the tree knows only reads", () => {
    for (const command of [
      "uptime",
      "pm list packages -3",
      "dumpsys battery",
      "ls -l /sdcard",
      "getprop ro.product.model",
      "settings get global adb_enabled",
      "wm size",
    ])
      expect(reads(command), command).toBe(true);
  });
  it("reviews everything else", () => {
    for (const command of [
      "rm -rf /sdcard",
      "pm uninstall org.demo",
      "settings put global adb_enabled 0",
      "reboot",
      "uptime; reboot",
      "uptime && reboot",
      "cat /sdcard/a > /sdcard/b",
      "ls $(reboot)",
      "ls `reboot`",
      "something unknown",
      "wm size 1920x1080",
      "logcat",
      "ping 1.1.1.1",
      "top",
      "find /sdcard -delete",
      "find /sdcard -name x -delete",
      "",
    ])
      expect(reads(command), command).toBe(false);
  });
});

describe("the scrollback", () => {
  it("drops the oldest results once it's full, and always keeps the newest", () => {
    const entry = (id: number, size: number) => ({
      id,
      command: "x",
      output: "a".repeat(size),
      failed: false,
    });
    const kept = keep("tv", [entry(1, 600_000), entry(2, 600_000)]);
    expect(kept.map((e) => e.id)).toEqual([2]);
    expect(keep("tv", [entry(3, 2_000_000)]).map((e) => e.id)).toEqual([3]);
    expect(scrollback("tv").map((e) => e.id)).toEqual([3]);
    keep("tv", []);
  });
  it("takes the heading off a stored result", () => {
    expect(outputOf("$ uptime\n\nup 3 days\n")).toBe("up 3 days\n");
    expect(outputOf("$ sync\n\n")).toBe("");
  });
});

const device: Device = {
  id: "session",
  deviceId: `device-${Math.random()}`,
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
function platform(output = "$ uptime\n\nup 3 days\n") {
  const query = vi.fn(async (_: unknown, q: Query) => {
    if (q.kind === "launchableApps")
      return [{ component: "org.demo/.Main", label: "Player" }];
    if (q.kind === "shellList")
      return q.source === "commands"
        ? ["pm", "uptime", "reboot", "settings", "ping"].map((value) => ({
            value,
            about: "",
          }))
        : q.source === "packages"
          ? [
              { value: "org.demo", about: "" },
              { value: "org.other", about: "" },
            ]
          : [{ value: `${q.source}-row`, about: "" }];
    return {};
  });
  const prepare = vi.fn(async () => ({
    id: "review",
    commands: ["uptime"],
    target: {},
    losses: [],
    title: "Run",
    effect: "",
    currentState: "",
    reversible: false,
  }));
  const apply = vi.fn(async () => ({ status: "sent", output, error: null }));
  return { query, prepare, apply, discard: vi.fn(async () => {}) };
}
function show(api = platform()) {
  render(ShellTool, {
    device: { ...device, deviceId: `d-${Math.random()}` },
    api: api as unknown as Platform,
  });
  return api;
}
const field = () => screen.getByRole("button", { name: "Command" });
const row = (text: string) =>
  screen.findByRole("option", { name: new RegExp(`^${text}`) });

describe("the Shell page", () => {
  it("starts in Normal, with no typing needed and an empty window", async () => {
    show();
    expect(screen.getByRole("button", { name: "Normal" }).className).toContain(
      "chosen",
    );
    expect(field()).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.getByText("Output appears here.")).toBeTruthy();
    expect(
      (screen.getByRole("button", { name: "Run" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });
  it("opens the words with what each does, and hides what this TV doesn't have", async () => {
    const api = show();
    await waitFor(() =>
      expect(api.query).toHaveBeenCalledWith(expect.anything(), {
        kind: "shellList",
        source: "commands",
        argument: null,
      }),
    );
    await new Promise((r) => setTimeout(r));
    await fireEvent.click(field());
    expect(await row("pm")).toBeTruthy();
    expect(
      screen.getByText("Apps: list, permissions, disable, uninstall."),
    ).toBeTruthy();
    expect(screen.queryByRole("option", { name: /^dmesg/ })).toBeNull();
  });
  it("changes the dropdown after each word, and runs what only reads straight away", async () => {
    const api = show();
    await fireEvent.click(field());
    await fireEvent.click(await row("uptime"));
    await fireEvent.click(screen.getByRole("button", { name: "Run it now" }));
    await waitFor(() => expect(api.apply).toHaveBeenCalledWith("review"));
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "shell",
      command: "uptime",
    });
    expect(await screen.findByText(/up 3 days/)).toBeTruthy();
  });
  it("fills a list from the TV, with app names where they're known", async () => {
    show();
    await fireEvent.click(field());
    await fireEvent.click(await row("pm"));
    await fireEvent.click(await row("path"));
    expect(await row("org.demo")).toBeTruthy();
    expect(await screen.findByText("Player")).toBeTruthy();
  });
  it("sends anything that changes the TV to a review, and doesn't apply it", async () => {
    const api = show();
    await fireEvent.click(field());
    await fireEvent.click(await row("reboot"));
    await fireEvent.click(
      await screen.findByRole("button", { name: "Run it now" }),
    );
    await waitFor(() => expect(api.prepare).toHaveBeenCalled());
    expect(api.apply).not.toHaveBeenCalled();
    expect(
      await screen.findByRole("button", { name: "Apply change" }),
    ).toBeTruthy();
  });
  it("shows a helper instead of a list when only the user can know the answer", async () => {
    const api = show();
    await fireEvent.click(field());
    await fireEvent.click(await row("ping"));
    await fireEvent.click(await row("Another address"));
    const input = await screen.findByRole("textbox", { name: "Host" });
    expect(screen.getByText("An IP address or a host name.")).toBeTruthy();
    expect(screen.getByText("1.1.1.1")).toBeTruthy();
    await fireEvent.click(
      screen.getByRole("button", { name: "Show the TV's address" }),
    );
    await waitFor(() =>
      expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
        kind: "shell",
        command: "ip -o addr show",
      }),
    );
    await fireEvent.input(input, { target: { value: "example.org" } });
    await fireEvent.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() =>
      expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
        kind: "shell",
        command: "ping -c 4 -W 2 example.org",
      }),
    );
  });
  it("removes the last word and goes back to its choices", async () => {
    show();
    await fireEvent.click(field());
    await fireEvent.click(await row("pm"));
    expect(await row("path")).toBeTruthy();
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove the last word" }),
    );
    expect(await row("uptime")).toBeTruthy();
  });
  it("lets someone type in Advanced, and reviews what it doesn't recognise", async () => {
    const api = show();
    await fireEvent.click(screen.getByRole("button", { name: "Advanced" }));
    const input = screen.getByRole("textbox", { name: "Command" });
    await fireEvent.input(input, {
      target: { value: "pm disable-user org.demo" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() => expect(api.prepare).toHaveBeenCalled());
    expect(api.apply).not.toHaveBeenCalled();
  });
  it("runs a typed command it knows only reads", async () => {
    const api = show();
    await fireEvent.click(screen.getByRole("button", { name: "Advanced" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Command" }), {
      target: { value: "uptime" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() => expect(api.apply).toHaveBeenCalled());
  });
  it("shows a failed command's exit code and output in the window", async () => {
    const api = platform("$ grep x /y\n\ngrep: /y: No such file\n");
    api.apply.mockResolvedValue({
      status: "failed",
      output: "$ grep x /y\n\ngrep: /y: No such file\n",
      error: "The TV reported exit code 2.",
    } as never);
    show(api);
    await fireEvent.click(screen.getByRole("button", { name: "Advanced" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Command" }), {
      target: { value: "uptime" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Run" }));
    expect(await screen.findByText("Exit code 2")).toBeTruthy();
    expect(screen.getByText(/No such file/)).toBeTruthy();
  });
  it("puts an earlier command back on the line", async () => {
    show();
    await fireEvent.click(screen.getByRole("button", { name: "Advanced" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Command" }), {
      target: { value: "uptime" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Run" }));
    await fireEvent.click(
      await screen.findByRole("button", { name: "Edit uptime" }),
    );
    expect(
      (screen.getByRole("textbox", { name: "Command" }) as HTMLInputElement)
        .value,
    ).toBe("uptime");
  });
});
