/**
 * Every word Normal mode offers on the Shell page. A command built from these always finishes on
 * its own: Normal mode can't wait for input and a command stops after 30 seconds, so `logcat` is
 * always a snapshot, `ping` always counts and `top` always runs once. A word that changes anything
 * says so with `effect`, and everything else must only read, because reads run without a review.
 */
import type { Effect, Helper, Step, Word } from "./shell-line";

const prefix = "/data/data/com.termux/files/usr";
const home = "/data/data/com.termux/files/home";
/** Termux's own programs only work with Termux's environment, which `run-as` doesn't set up. */
export const inTermux = (inner: string) =>
  `run-as com.termux ${prefix}/bin/env -i HOME=${home} PREFIX=${prefix} PATH=${prefix}/bin TMPDIR=${prefix}/tmp LANG=en_US.UTF-8 TERM=dumb ${prefix}/bin/bash -c ${quoteAll(inner)}`;
const quoteAll = (value: string) => `'${value.replaceAll("'", `'"'"'`)}'`;

const words = (prompt: string, list: Word[], extra: Partial<Step> = {}): Step =>
  ({ kind: "words", prompt, words: list, ...extra }) as Step;

const w = (
  word: string,
  about: string,
  more: Omit<Partial<Word>, "next"> & { next?: Word["next"] } = {},
): Word => ({ word, about, ...more });
const change = (
  word: string,
  about: string,
  more: Omit<Partial<Word>, "next"> & { next?: Word["next"] } = {},
): Word => ({ word, about, effect: "change", ...more });
const danger = (
  word: string,
  about: string,
  warning: string,
  more: Omit<Partial<Word>, "next"> & { next?: Word["next"] } = {},
): Word => ({ word, about, effect: "danger", warning, ...more });

const text = (
  prompt: string,
  about: string,
  example: string,
  more: Partial<Helper> = {},
  step: Partial<Step> = {},
): Step =>
  ({
    kind: "text",
    prompt,
    helper: { label: prompt, about, example, ...more },
    ...step,
  }) as Step;
const number = (prompt: string, about: string, example: string): Step =>
  text(prompt, about, example, { pattern: /^\d{1,10}$/ });

const list = (
  prompt: string,
  source: Extract<Step, { kind: "list" }>["source"],
  more: Partial<Extract<Step, { kind: "list" }>> = {},
): Step => ({ kind: "list", prompt, source, ...more }) as Step;

const appHelper: Helper = {
  label: "Package name",
  about: "Package names look like com.example.app.",
  example: "com.android.vending",
  pattern: /^[\w.]+$/,
  find: { label: "List every app", command: "pm list packages" },
};
const app = (more: Partial<Extract<Step, { kind: "list" }>> = {}): Step =>
  list("Which app", "packages", {
    remember: "package",
    other: appHelper,
    ...more,
  });
const permission = (more: Partial<Step> = {}): Step =>
  list("Which permission", "permissions", {
    argument: "package",
    other: {
      label: "Permission name",
      about: "Permission names look like android.permission.CAMERA.",
      example: "android.permission.CAMERA",
      pattern: /^[\w.]+$/,
      find: {
        label: "List every permission",
        command: "pm list permissions -g",
      },
    },
    ...more,
  } as Partial<Extract<Step, { kind: "list" }>>);
const pathHelper: Helper = {
  label: "Path",
  about: "A full path, starting with a slash.",
  example: "/sdcard/Download",
  pattern: /^\//,
  find: {
    label: "List your shared storage",
    command: "ls -la /sdcard/",
  },
};
const path = (prompt: string, more: Partial<Step> = {}): Step =>
  ({ kind: "path", prompt, other: pathHelper, ...more }) as Step;
const folder = (prompt: string, more: Partial<Step> = {}): Step =>
  path(prompt, { folders: true, ...more } as Partial<Step>);

/** Options that can be picked in any order, each once, with a row to move on. */
function flags(
  prompt: string,
  options: Word[],
  then: Step | null,
  moveOn = "No more options",
): Step {
  const rows: Word[] = options.map((option) => {
    const rest = () =>
      flags(
        prompt,
        options.filter((o) => o !== option),
        then,
        moveOn,
      );
    const value =
      typeof option.next === "function" ? option.next() : option.next;
    // An option that takes a value asks for it first, then the options carry on.
    return { ...option, next: value ? { ...value, next: rest } : rest };
  });
  if (then)
    rows.unshift({
      word: moveOn,
      about: "Go on to the next word.",
      as: "",
      next: then,
    });
  return words(prompt, rows, { optional: !then });
}
const optionalFlags = (prompt: string, options: Word[], then?: Step) =>
  flags(prompt, options, then ?? null);

const packageFilters: Word[] = [
  w("-3", "Only apps you added."),
  w("-s", "Only apps that came with the TV."),
  w("-d", "Only disabled apps."),
  w("-e", "Only enabled apps."),
  w("-f", "Add where each app's file is."),
  w("-i", "Add what installed each app."),
  w("-u", "Include apps removed from this profile."),
  w("-U", "Add each app's user ID."),
  w("--show-versioncode", "Add each app's version code.", { minSdk: 28 }),
];

const pm: Step = words("What to do with apps", [
  w("list", "List apps, permissions and more.", {
    group: "Look things up",
    next: words("What to list", [
      w("packages", "Installed apps, as package names.", {
        next: optionalFlags("Narrow the list", packageFilters),
      }),
      w("permissions", "Every permission on the TV.", {
        next: optionalFlags("Narrow the list", [
          w("-g", "Group them."),
          w("-f", "Show full details."),
          w("-s", "Short summary."),
          w("-d", "Only dangerous permissions."),
          w("-u", "Only those users see."),
        ]),
      }),
      w("permission-groups", "Permission groups."),
      w("features", "What the TV's hardware and software support."),
      w("libraries", "Shared libraries on the TV."),
      w("users", "Profiles on this TV."),
      w("instrumentation", "Test packages."),
    ]),
  }),
  w("path", "Where an app's files are.", {
    group: "Look things up",
    next: app(),
  }),
  w("dump", "Everything Android knows about an app.", {
    group: "Look things up",
    next: app(),
  }),
  w("get-install-location", "Where new apps go.", { group: "Look things up" }),
  w("get-max-users", "How many profiles this TV allows.", {
    group: "Look things up",
  }),
  w("has-feature", "Whether the TV has a feature.", {
    group: "Look things up",
    next: list("Which feature", "features"),
  }),
  w("get-privapp-permissions", "System permissions an app is allowed.", {
    group: "Look things up",
    minSdk: 28,
    next: app(),
  }),
  change("disable-user", "Switch an app off. Its data stays.", {
    as: "disable-user --user {user}",
    group: "Switch apps on and off",
    warning: "Switching off Settings or your launcher can leave you stuck.",
    next: app(),
  }),
  change("enable", "Switch an app back on.", {
    as: "enable --user {user}",
    group: "Switch apps on and off",
    next: app({ source: "disabledPackages" }),
  }),
  change("default-state", "Put an app back to how it came.", {
    as: "default-state --user {user}",
    group: "Switch apps on and off",
    next: app(),
  }),
  change("suspend", "Pause an app. It shows greyed out.", {
    as: "suspend --user {user}",
    group: "Switch apps on and off",
    next: app(),
  }),
  change("unsuspend", "Undo pausing an app.", {
    as: "unsuspend --user {user}",
    group: "Switch apps on and off",
    next: app(),
  }),
  danger(
    "uninstall",
    "Delete an app you added.",
    "The app and its data are deleted.",
    {
      group: "Remove things",
      next: flags(
        "Options",
        [w("-k", "Keep its data and cache.")],
        list("Which app", "userPackages", { other: appHelper }),
      ),
    },
  ),
  danger(
    "uninstall --user",
    "Remove a preinstalled app from this profile.",
    "Its data is deleted. Removing a system app can break the TV, so check its rating in Apps first.",
    { as: "uninstall --user {user}", group: "Remove things", next: app() },
  ),
  change("install-existing", "Bring back an app removed from this profile.", {
    as: "install-existing --user {user}",
    group: "Remove things",
    next: text("Package name", appHelper.about, appHelper.example, {
      find: { label: "List removed apps", command: "pm list packages -u" },
    }),
  }),
  danger(
    "clear",
    "Delete an app's data.",
    "Sign-ins, settings and downloads in this app are deleted.",
    {
      as: "clear --user {user}",
      group: "Remove things",
      next: app(),
    },
  ),
  change("trim-caches", "Free up space by clearing app caches.", {
    group: "Remove things",
    next: words("How much free space to aim for", [
      w("500M", "500 MB."),
      w("1G", "1 GB."),
      w("4G", "4 GB."),
      w("100G", "As much as possible."),
    ]),
  }),
  change("grant", "Give an app a permission.", {
    group: "Permissions",
    next: app({ next: permission() } as Partial<
      Extract<Step, { kind: "list" }>
    >),
  }),
  change("revoke", "Take a permission away from an app.", {
    group: "Permissions",
    next: app({ next: permission() } as Partial<
      Extract<Step, { kind: "list" }>
    >),
  }),
  change("reset-permissions", "Put every app's permissions back.", {
    group: "Permissions",
  }),
  change("set-home-activity", "Choose the Home app.", {
    group: "Other changes",
    warning:
      "Use Tools > Home app instead, which also checks Home really changed.",
    next: text(
      "Launcher component",
      "An app and its activity, joined by a slash.",
      "com.spocky.projengmenu/.ui.home.MainActivity",
      {
        find: {
          label: "List launchers",
          command:
            "cmd package query-activities --brief -a android.intent.action.MAIN -c android.intent.category.HOME",
        },
      },
    ),
  }),
  change("set-install-location", "Choose where new apps go.", {
    group: "Other changes",
    next: words("Where", [
      w("0", "Let Android decide."),
      w("1", "Internal storage."),
      w("2", "External storage."),
    ]),
  }),
  change("compile", "Rebuild an app so it starts faster.", {
    as: "compile -m speed -f",
    group: "Other changes",
    next: app(),
  }),
]);

const settingsScreens: Word[] = [
  ["android.settings.SETTINGS", "The main settings screen."],
  ["android.settings.APPLICATION_SETTINGS", "Apps."],
  ["android.settings.MANAGE_ALL_APPLICATIONS_SETTINGS", "All apps."],
  ["android.settings.WIFI_SETTINGS", "Wi-Fi."],
  ["android.settings.BLUETOOTH_SETTINGS", "Bluetooth and remotes."],
  ["android.settings.DISPLAY_SETTINGS", "Display."],
  ["android.settings.SOUND_SETTINGS", "Sound."],
  ["android.settings.DATE_SETTINGS", "Date and time."],
  ["android.settings.LOCALE_SETTINGS", "Language."],
  ["android.settings.INPUT_METHOD_SETTINGS", "Keyboards."],
  ["android.settings.ACCESSIBILITY_SETTINGS", "Accessibility."],
  ["android.settings.SECURITY_SETTINGS", "Security."],
  ["android.settings.INTERNAL_STORAGE_SETTINGS", "Storage."],
  ["android.settings.DEVICE_INFO_SETTINGS", "About this TV."],
  ["android.settings.APPLICATION_DEVELOPMENT_SETTINGS", "Developer options."],
  ["android.settings.USAGE_ACCESS_SETTINGS", "Usage access."],
  [
    "android.settings.action.MANAGE_OVERLAY_PERMISSION",
    "Display over other apps.",
  ],
  ["android.settings.MANAGE_UNKNOWN_APP_SOURCES", "Install unknown apps."],
  ["android.settings.ZEN_MODE_SETTINGS", "Do not disturb."],
  ["android.settings.DREAM_SETTINGS", "Screensaver."],
].map(([word, about]) => w(word, about));

const am: Step = words("What to do with running apps", [
  change("start", "Open an app or a screen on the TV.", {
    group: "Open things",
    next: words("What to open", [
      change("-n", "A specific screen of an app.", {
        next: app({
          silent: true,
          next: list("Which screen", "activities", {
            argument: "package",
            other: {
              label: "Activity",
              about: "An app and its activity, joined by a slash.",
              example: "com.android.tv.settings/.MainSettings",
            },
          }),
        } as Partial<Extract<Step, { kind: "list" }>>),
        as: "-n",
      }),
      change("Home screen", "Go to the launcher.", {
        as: "-W -c android.intent.category.HOME -a android.intent.action.MAIN",
      }),
      change(
        "Settings screen",
        "Open a settings screen, including hidden ones.",
        {
          as: "-a",
          next: words("Which screen", settingsScreens),
        },
      ),
      change("Web page", "Open a link in the TV's browser.", {
        as: "-a android.intent.action.VIEW -d",
        next: text("Link", "A full web address.", "https://example.com", {
          pattern: /^[a-z][a-z0-9+.-]*:/i,
        }),
      }),
    ]),
  }),
  change("broadcast", "Send a message to apps on the TV.", {
    as: "broadcast -a",
    group: "Open things",
    next: text(
      "Action",
      "The action name the app listens for.",
      "android.intent.action.BOOT_COMPLETED",
      {
        find: {
          label: "List what's listening",
          command: "dumpsys activity broadcasts | grep -m 40 Action",
        },
      },
    ),
  }),
  change("force-stop", "Close an app now. It can start again.", {
    group: "Stop things",
    next: app(),
  }),
  change("kill", "End an app's background work, if it's idle.", {
    group: "Stop things",
    next: app(),
  }),
  change("kill-all", "End all background apps.", { group: "Stop things" }),
  change("crash", "Make an app crash, for testing.", {
    group: "Stop things",
    next: app(),
  }),
  w("get-current-user", "Which profile is active.", {
    group: "Look things up",
  }),
  w("get-config", "The TV's current configuration.", {
    group: "Look things up",
  }),
  w("stack list", "Open apps and their windows.", { group: "Look things up" }),
  w("get-inactive", "Whether Android thinks an app is idle.", {
    group: "Look things up",
    next: app(),
  }),
  w("get-standby-bucket", "How often an app may run.", {
    group: "Look things up",
    minSdk: 28,
    next: app(),
  }),
  change("set-standby-bucket", "Change how often an app may run.", {
    group: "Change how apps run",
    minSdk: 28,
    next: app({
      next: words("How often", [
        w("active", "Whenever it wants."),
        w("working_set", "Often."),
        w("frequent", "Regularly."),
        w("rare", "Rarely."),
      ]),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  change("set-inactive", "Tell Android an app is idle or not.", {
    group: "Change how apps run",
    next: app({
      next: words("Idle", [
        w("true", "Treat it as idle."),
        w("false", "Treat it as active."),
      ]),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  change("idle-maintenance", "Run Android's idle clean-up now.", {
    group: "Change how apps run",
  }),
  change("send-trim-memory", "Ask an app to free up memory.", {
    group: "Change how apps run",
    next: app({
      next: words("How hard", [
        w("RUNNING_LOW", "A little."),
        w("MODERATE", "More."),
        w("COMPLETE", "As much as it can."),
      ]),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  change("switch-user", "Switch to another profile.", {
    group: "Change how apps run",
    next: list("Which profile", "users"),
  }),
]);

const opWords: Word[] = [
  ["CAMERA", "Use the camera."],
  ["RECORD_AUDIO", "Use the microphone."],
  ["COARSE_LOCATION", "Rough location."],
  ["FINE_LOCATION", "Exact location."],
  ["READ_EXTERNAL_STORAGE", "Read shared storage."],
  ["WRITE_EXTERNAL_STORAGE", "Write to shared storage."],
  ["SYSTEM_ALERT_WINDOW", "Draw over other apps."],
  ["WRITE_SETTINGS", "Change system settings."],
  ["GET_USAGE_STATS", "See which apps you use."],
  ["RUN_IN_BACKGROUND", "Keep running in the background."],
  ["RUN_ANY_IN_BACKGROUND", "Run in the background, any kind."],
  ["REQUEST_INSTALL_PACKAGES", "Install other apps."],
  ["MANAGE_EXTERNAL_STORAGE", "All files access."],
  ["POST_NOTIFICATION", "Show notifications."],
  ["PICTURE_IN_PICTURE", "Picture in picture."],
].map(([word, about]) => w(word, about));
const opList = (next?: Step) =>
  list("Which permission", "appOps", {
    argument: "package",
    extra: opWords.map((op) => ({ ...op, next })),
    next,
    other: {
      label: "Operation",
      about: "Written in capitals.",
      example: "CAMERA",
      pattern: /^[A-Z_]+$/,
    },
  } as Partial<Extract<Step, { kind: "list" }>>);
const appops: Step = words("What to do with app access", [
  w("get", "What an app is allowed to do.", { next: app() }),
  w("query-op", "Which apps are allowed or denied something.", {
    next: words(
      "Which permission",
      opWords.map((op) => ({
        ...op,
        next: words("Which answer", [
          w("allow", "Apps that are allowed."),
          w("ignore", "Apps that are silently refused."),
          w("deny", "Apps that are refused."),
        ]),
      })),
    ),
  }),
  change("set", "Allow or refuse something for an app.", {
    next: app({
      next: opList(
        words("Which answer", [
          w("allow", "Let the app do this."),
          w("ignore", "Refuse without telling it."),
          w("deny", "Refuse and tell it."),
          w("default", "Back to Android's default."),
        ]),
      ),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  change("reset", "Put an app's access back to default.", { next: app() }),
]);

const namespaces = (
  prompt: string,
  key: (space: string) => Step | undefined,
  effect?: Effect,
): Step =>
  words(
    prompt,
    [
      ["system", "Everyday settings for this profile."],
      ["secure", "Sensitive settings for this profile."],
      ["global", "Settings shared by the whole TV."],
    ].map(([space, about]) => w(space, about, { effect, next: key(space) })),
  );
const settingKey = (space: string, more: Partial<Step> = {}) =>
  list("Which setting", "settings", {
    argument: space,
    other: {
      label: "Setting name",
      about: "Written in lower case, with underscores.",
      example: "screen_off_timeout",
      pattern: /^[\w.-]+$/,
      find: { label: "List these settings", command: `settings list ${space}` },
    },
    ...more,
  } as Partial<Extract<Step, { kind: "list" }>>);
const settings: Step = words("What to do with a setting", [
  w("get", "Read one setting.", {
    next: namespaces("Which group", (space) => settingKey(space)),
  }),
  w("list", "Read a whole group of settings.", {
    next: namespaces("Which group", () => undefined),
  }),
  change("put", "Change a setting.", {
    warning:
      "A wrong value can change how the TV behaves. Tools > Settings has the safe ones.",
    next: namespaces("Which group", (space) =>
      settingKey(space, {
        next: text(
          "New value",
          "The value to store. Pick the same setting with get to see the current one.",
          "1",
          {
            find: {
              label: "Show these settings",
              command: `settings list ${space}`,
            },
          },
        ),
      }),
    ),
  }),
  danger(
    "delete",
    "Delete a setting.",
    "Android goes back to its default, and the old value is lost.",
    {
      next: namespaces("Which group", (space) => settingKey(space)),
    },
  ),
]);

const size = /^\d{2,5}x\d{2,5}$/;
const wm: Step = words("Screen size and text size", [
  w("size", "Show the screen size."),
  w("density", "Show the text and icon size."),
  change("size set", "Change the screen size.", {
    as: "size",
    next: text("Size", "Width, an x, then height.", "1920x1080", {
      pattern: size,
      find: { label: "Show the current size", command: "wm size" },
    }),
  }),
  change("size reset", "Back to the TV's own size."),
  change("density set", "Change the text and icon size.", {
    as: "density",
    next: number(
      "Density",
      "Higher numbers look bigger. TVs usually use 213 to 320.",
      "240",
    ),
  }),
  change("density reset", "Back to the TV's own text size."),
  change("dismiss-keyguard", "Dismiss the lock screen."),
]);

const digits = Array.from({ length: 10 }, (_, n) =>
  w(`KEYCODE_${n}`, `The ${n} key.`, { group: "Letters and numbers" }),
);
const letters = Array.from({ length: 26 }, (_, n) => {
  const letter = String.fromCharCode(65 + n);
  return w(`KEYCODE_${letter}`, `The ${letter} key.`, {
    group: "Letters and numbers",
  });
});
const functionKeys = Array.from({ length: 12 }, (_, n) =>
  w(`KEYCODE_F${n + 1}`, `The F${n + 1} key.`, { group: "Other keys" }),
);
const keyRows = (group: string, keys: [string, string][]) =>
  keys.map(([key, about]) => w(`KEYCODE_${key}`, about, { group }));
export const keycodes: Word[] = [
  ...keyRows("Navigation", [
    ["HOME", "Go to the home screen."],
    ["BACK", "Go back one screen."],
    ["DPAD_UP", "Up."],
    ["DPAD_DOWN", "Down."],
    ["DPAD_LEFT", "Left."],
    ["DPAD_RIGHT", "Right."],
    ["DPAD_CENTER", "Select."],
    ["ENTER", "Enter."],
    ["MENU", "Menu."],
    ["SETTINGS", "Open the TV's settings."],
    ["SEARCH", "Search."],
    ["GUIDE", "Open the programme guide."],
    ["INFO", "Show information."],
    ["APP_SWITCH", "Open recent apps."],
    ["NOTIFICATION", "Open notifications."],
    ["ASSIST", "Open the assistant."],
    ["BOOKMARK", "Bookmark."],
    ["PAGE_UP", "Page up."],
    ["PAGE_DOWN", "Page down."],
    ["TAB", "Tab."],
    ["ESCAPE", "Escape."],
    ["MOVE_HOME", "Jump to the start."],
    ["MOVE_END", "Jump to the end."],
  ]),
  ...keyRows("Sound and picture", [
    ["VOLUME_UP", "Louder."],
    ["VOLUME_DOWN", "Quieter."],
    ["VOLUME_MUTE", "Mute."],
    ["MUTE", "Mute the microphone."],
    ["BRIGHTNESS_UP", "Brighter."],
    ["BRIGHTNESS_DOWN", "Dimmer."],
    ["CAPTIONS", "Turn captions on or off."],
    ["TV_ZOOM_MODE", "Change the zoom."],
    ["TV_AUDIO_DESCRIPTION", "Audio description."],
    ["TV_AUDIO_DESCRIPTION_MIX_UP", "Audio description louder."],
    ["TV_AUDIO_DESCRIPTION_MIX_DOWN", "Audio description quieter."],
  ]),
  ...keyRows("Playback", [
    ["MEDIA_PLAY_PAUSE", "Play or pause."],
    ["MEDIA_PLAY", "Play."],
    ["MEDIA_PAUSE", "Pause."],
    ["MEDIA_STOP", "Stop."],
    ["MEDIA_NEXT", "Next."],
    ["MEDIA_PREVIOUS", "Previous."],
    ["MEDIA_FAST_FORWARD", "Fast forward."],
    ["MEDIA_REWIND", "Rewind."],
    ["MEDIA_RECORD", "Record."],
    ["MEDIA_EJECT", "Eject."],
    ["MEDIA_AUDIO_TRACK", "Change the audio track."],
    ["MEDIA_SKIP_FORWARD", "Skip forward."],
    ["MEDIA_SKIP_BACKWARD", "Skip back."],
    ["MEDIA_STEP_FORWARD", "One frame forward."],
    ["MEDIA_STEP_BACKWARD", "One frame back."],
  ]),
  ...keyRows("Power and inputs", [
    ["POWER", "Power."],
    ["SLEEP", "Put the TV to sleep."],
    ["SOFT_SLEEP", "Dim for a light sleep."],
    ["WAKEUP", "Wake the TV."],
    ["TV", "Go to live TV."],
    ["TV_POWER", "Power on a connected TV."],
    ["TV_INPUT", "Show the input list."],
    ["TV_INPUT_HDMI_1", "HDMI 1."],
    ["TV_INPUT_HDMI_2", "HDMI 2."],
    ["TV_INPUT_HDMI_3", "HDMI 3."],
    ["TV_INPUT_HDMI_4", "HDMI 4."],
    ["TV_INPUT_ANTENNA", "Antenna."],
    ["TV_INPUT_COMPOSITE_1", "Composite 1."],
    ["TV_INPUT_COMPONENT_1", "Component 1."],
    ["TV_INPUT_VGA_1", "VGA."],
    ["STB_POWER", "Set-top box power."],
    ["STB_INPUT", "Set-top box input."],
    ["AVR_POWER", "Receiver power."],
    ["AVR_INPUT", "Receiver input."],
  ]),
  ...keyRows("Live TV", [
    ["CHANNEL_UP", "Next channel."],
    ["CHANNEL_DOWN", "Previous channel."],
    ["LAST_CHANNEL", "Back to the last channel."],
    ["DVR", "Recordings."],
    ["TV_TELETEXT", "Teletext."],
    ["TV_RADIO_SERVICE", "Radio channels."],
    ["TV_DATA_SERVICE", "Data service."],
    ["TV_CONTENTS_MENU", "Contents menu."],
    ["TV_MEDIA_CONTEXT_MENU", "Media menu."],
    ["TV_TIMER_PROGRAMMING", "Timer."],
    ["TV_ANTENNA_CABLE", "Antenna or cable."],
    ["TV_NETWORK", "Network."],
    ["TV_SATELLITE", "Satellite."],
    ["TV_TERRESTRIAL_DIGITAL", "Digital terrestrial."],
    ["TV_TERRESTRIAL_ANALOG", "Analogue terrestrial."],
    ["PROG_RED", "Red button."],
    ["PROG_GREEN", "Green button."],
    ["PROG_YELLOW", "Yellow button."],
    ["PROG_BLUE", "Blue button."],
    ["NUMPAD_ENTER", "Number pad enter."],
  ]),
  ...digits,
  ...letters,
  ...keyRows("Other keys", [
    ["SPACE", "Space."],
    ["DEL", "Delete the last letter."],
    ["FORWARD_DEL", "Delete the next letter."],
    ["COMMA", "Comma."],
    ["PERIOD", "Full stop."],
    ["MINUS", "Minus."],
    ["PLUS", "Plus."],
    ["EQUALS", "Equals."],
    ["SLASH", "Slash."],
    ["BACKSLASH", "Backslash."],
    ["SEMICOLON", "Semicolon."],
    ["APOSTROPHE", "Apostrophe."],
    ["AT", "At sign."],
    ["STAR", "Star."],
    ["POUND", "Hash."],
    ["GRAVE", "Backtick."],
    ["LEFT_BRACKET", "Left bracket."],
    ["RIGHT_BRACKET", "Right bracket."],
    ["CAPS_LOCK", "Caps lock."],
    ["SHIFT_LEFT", "Shift."],
    ["CTRL_LEFT", "Control."],
    ["ALT_LEFT", "Alt."],
    ["META_LEFT", "Windows or Command key."],
    ["EXPLORER", "Open the browser."],
    ["CALCULATOR", "Open the calculator."],
    ["CALENDAR", "Open the calendar."],
    ["MUSIC", "Open music."],
    ["CONTACTS", "Open contacts."],
    ["ENVELOPE", "Open email."],
    ["CAMERA", "Open the camera."],
    ["CUT", "Cut."],
    ["COPY", "Copy."],
    ["PASTE", "Paste."],
    ["BUTTON_A", "Controller button A."],
    ["BUTTON_B", "Controller button B."],
    ["BUTTON_X", "Controller button X."],
    ["BUTTON_Y", "Controller button Y."],
    ["BUTTON_START", "Controller Start."],
    ["BUTTON_SELECT", "Controller Select."],
    ["BUTTON_MODE", "Controller mode button."],
    ["BUTTON_L1", "Controller left shoulder."],
    ["BUTTON_R1", "Controller right shoulder."],
  ]),
  ...functionKeys,
];

const keyStep = words("Which key", keycodes);
const coordinate = (label: string, then?: Step): Step =>
  text(
    label,
    "A position in pixels, counted from the top left.",
    "100",
    {
      pattern: /^\d{1,5}$/,
      find: { label: "Show the screen size", command: "wm size" },
    },
    { next: then } as Partial<Step>,
  );
const input: Step = words("Send input to the TV", [
  change("keyevent", "Press a key on the remote.", { next: keyStep }),
  change("keyevent --longpress", "Hold a key down.", {
    next: keyStep,
    minSdk: 26,
  }),
  change("text", "Type into the field on the TV.", {
    next: text(
      "Text to type",
      "The TV types this into whatever field is selected.",
      "hello world",
      {
        encode: (value) => value.replaceAll(" ", "%s"),
      },
    ),
  }),
  change("tap", "Click a spot on the screen.", {
    next: coordinate("Across", coordinate("Down")),
  }),
  change("swipe", "Drag between two spots.", {
    next: coordinate(
      "From across",
      coordinate("From down", coordinate("To across", coordinate("To down"))),
    ),
  }),
]);

const logBuffers = [
  w("-b main", "Normal app messages."),
  w("-b system", "System messages."),
  w("-b crash", "Crashes only."),
  w("-b events", "System events."),
  w("-b all", "Every buffer."),
];
const logLevels = [
  w("*:E", "Errors only.", { as: "'*:E'" }),
  w("*:W", "Warnings and errors.", { as: "'*:W'" }),
  w("*:I", "Information and above.", { as: "'*:I'" }),
  w("*:D", "Debug and above.", { as: "'*:D'" }),
];
const logFilters = optionalFlags("Narrow the log", [
  ...logBuffers,
  ...logLevels,
  w("-v time", "Add the time to each line."),
  w("-v threadtime", "Add time, process and thread."),
  w("--pid", "Only one running program.", {
    next: list("Which program", "processes"),
  }),
  w("-s", "Only one tag.", {
    next: text(
      "Tag",
      "The word at the start of the log lines you want.",
      "ActivityManager",
      {
        find: { label: "Show recent lines", command: "logcat -d -t 20" },
      },
    ),
  }),
]);
const logcat: Step = words("Read the TV's log", [
  w("-d -t 100", "The last 100 lines.", { next: logFilters }),
  w("-d -t 500", "The last 500 lines.", { next: logFilters }),
  w("-d -t 2000", "The last 2,000 lines.", { next: logFilters }),
  w("-g", "How big the log buffers are."),
  w("-S", "Which programs log the most."),
  change("-c", "Empty the log."),
]);

/** dumpsys services people ask for most. The rest come from the TV, named but unexplained. */
export const serviceAbout: Record<string, string> = {
  accessibility: "Accessibility services.",
  account: "Accounts on the TV.",
  activity: "Running apps and activities.",
  alarm: "Alarms apps have set.",
  appops: "What apps have been doing with their access.",
  audio: "Sound output and volume.",
  battery: "Power status.",
  batterystats: "Power use by app.",
  bluetooth_manager: "Bluetooth and remotes.",
  connectivity: "Network connections.",
  cpuinfo: "Processor use.",
  dbinfo: "App databases.",
  device_policy: "Device owner policies.",
  deviceidle: "Idle and battery-saver state.",
  diskstats: "Disk use.",
  display: "Screens and modes.",
  dropbox: "Saved crash reports.",
  gfxinfo: "Graphics use by app.",
  hdmi_control: "HDMI CEC.",
  input: "Remotes and keyboards.",
  input_method: "Keyboards.",
  jobscheduler: "Scheduled background jobs.",
  media_session: "What's playing.",
  meminfo: "Memory use.",
  mount: "Storage volumes.",
  netstats: "Data use.",
  network_management: "Network interfaces.",
  notification: "Notifications.",
  overlay: "Theme overlays.",
  package: "Every app and permission.",
  power: "Sleep and wake.",
  procstats: "Long-term process statistics.",
  role: "Default apps, including Home.",
  sensorservice: "Sensors.",
  statusbar: "Status bar.",
  storagestats: "Storage by app.",
  SurfaceFlinger: "What's on screen.",
  thermalservice: "Temperature.",
  tv_input: "HDMI and tuner inputs.",
  uimode: "TV mode and dark mode.",
  usagestats: "App usage history.",
  usb: "USB state.",
  user: "Profiles.",
  voiceinteraction: "Voice assistant.",
  wifi: "Wi-Fi.",
  window: "Windows and focus.",
};
const dumpsysStep: Step = list("Which part of the system", "services", {
  describe: serviceAbout,
  extra: [
    w("package", "Everything Android knows about an app.", { next: app() }),
    w("meminfo", "Memory use, for an app or everything.", {
      next: list("Which app", "packages", { optional: true, other: appHelper }),
    }),
    w("gfxinfo", "Graphics use by an app.", { next: app() }),
    w("activity", "Running apps. Pick a section.", {
      next: words(
        "Which section",
        [
          w("activities", "Open activities."),
          w("services", "Running services."),
          w("broadcasts", "Recent broadcasts."),
          w("processes", "Running processes."),
          w("recents", "Recent apps."),
          w("top", "The app in front."),
        ],
        { optional: true } as Partial<Step>,
      ),
    }),
    w("window", "Windows. Pick a section.", {
      next: words(
        "Which section",
        [
          w("displays", "Screens."),
          w("windows", "Every window."),
          w("policy", "Window policy."),
        ],
        { optional: true } as Partial<Step>,
      ),
    }),
  ],
  other: {
    label: "Service",
    about: "The name of a system service.",
    example: "battery",
    find: { label: "List every service", command: "dumpsys -l" },
  },
});

const onOff = (enable = "enable", disable = "disable") => [
  change(enable, "Turn it on."),
  danger(
    disable,
    "Turn it off.",
    "Turning this off can cut this phone off from the TV.",
  ),
];
const svc: Step = words("Turn hardware on and off", [
  w("wifi", "Wi-Fi.", {
    effect: "change",
    next: words("Wi-Fi", onOff()),
  }),
  w("data", "Mobile data, where a TV has it.", {
    effect: "change",
    next: words("Mobile data", [
      change("enable", "Turn it on."),
      change("disable", "Turn it off."),
    ]),
  }),
  w("bluetooth", "Bluetooth.", {
    effect: "change",
    minSdk: 31,
    next: words("Bluetooth", [
      change("enable", "Turn it on."),
      danger(
        "disable",
        "Turn it off.",
        "A Bluetooth remote stops working until Bluetooth is on again.",
      ),
    ]),
  }),
  w("power stayon", "Keep the TV awake.", {
    effect: "change",
    next: words("Stay awake", [
      w("true", "Stay on until told otherwise."),
      w("false", "Sleep as normal."),
      w("usb", "Stay on while USB is connected."),
      w("ac", "Stay on while on mains power."),
    ]),
  }),
  danger(
    "power reboot",
    "Restart the TV.",
    "Anything playing stops, and you'll need to reconnect.",
  ),
  danger(
    "power shutdown",
    "Turn the TV off.",
    "You'll need the remote to turn it back on.",
  ),
  w("usb getFunctions", "What the USB port is doing."),
]);

const cmdWords: Word[] = [
  w("package", "The same as pm. Pick a word.", { next: pm }),
  w("appops", "The same as appops. Pick a word.", { next: appops }),
  w("wifi", "Wi-Fi, on Android 11 and later.", {
    minSdk: 30,
    next: words("What to do with Wi-Fi", [
      w("status", "The network in use and its signal."),
      w("list-networks", "Saved networks."),
      w("list-scan-results", "Networks nearby."),
      change("start-scan", "Look for networks."),
      change("set-wifi-enabled enabled", "Turn Wi-Fi on."),
      danger(
        "set-wifi-enabled disabled",
        "Turn Wi-Fi off.",
        "Turning this off can cut this phone off from the TV.",
      ),
    ]),
  }),
  w("uimode", "TV mode and dark mode.", {
    next: words("What to do", [
      w("night", "Show dark mode."),
      change("night yes", "Dark mode on."),
      change("night no", "Dark mode off."),
      change("night auto", "Dark mode by time of day."),
    ]),
  }),
  w("overlay", "Theme overlays.", {
    next: words("What to do", [w("list", "List overlays.")]),
  }),
  w("role", "Default apps, on Android 10 and later.", {
    minSdk: 29,
    next: words("What to do", [
      w("get-role-holders android.app.role.HOME", "Which app holds Home.", {
        minSdk: 31,
      }),
      w("holders android.app.role.HOME", "Which app holds Home.", {
        maxSdk: 30,
      }),
    ]),
  }),
  w("media_session", "What's playing, on Android 12 and later.", {
    minSdk: 31,
    next: words("What to do", [
      w("list-sessions", "Apps that are playing."),
      change("dispatch play-pause", "Play or pause."),
      change("dispatch next", "Next."),
      change("dispatch previous", "Previous."),
      change("dispatch mute", "Mute."),
    ]),
  }),
  w("deviceidle", "Idle mode and its exceptions.", {
    next: words("What to do", [
      w("whitelist", "Apps allowed to run when idle."),
      change("whitelist +", "Let an app run when idle.", {
        as: "whitelist",
        next: text(
          "Package name",
          "Written with a plus in front.",
          "+com.example.app",
          {
            pattern: /^\+[\w.]+$/,
            find: { label: "List every app", command: "pm list packages" },
          },
        ),
      }),
      w("get deep", "Current deep idle state."),
    ]),
  }),
  w("notification", "Notifications.", {
    next: words("What to do", [
      change("allow_listener", "Let an app read notifications.", {
        next: text(
          "Listener component",
          "An app and its service, joined by a slash.",
          "com.example.app/.Listener",
        ),
      }),
      change("disallow_listener", "Stop an app reading notifications.", {
        next: text(
          "Listener component",
          "An app and its service, joined by a slash.",
          "com.example.app/.Listener",
        ),
      }),
    ]),
  }),
  w("connectivity", "Network, on Android 11 and later.", {
    minSdk: 30,
    next: words("What to do", [
      w("airplane-mode", "Show whether airplane mode is on."),
      danger(
        "airplane-mode enable",
        "Turn airplane mode on.",
        "Wireless connections drop, including this phone's.",
      ),
      change("airplane-mode disable", "Turn airplane mode off."),
    ]),
  }),
  w("thermalservice", "Temperature, on Android 10 and later.", {
    minSdk: 29,
    next: words("What to do", [w("help", "Show its commands.")]),
  }),
];
const cmd: Step = list("Which system service", "cmdServices", {
  describe: serviceAbout,
  extra: cmdWords,
  next: words("What to ask it", [w("help", "Show what this service can do.")]),
  other: {
    label: "Service",
    about: "The name of a system service.",
    example: "package",
    find: { label: "List every service", command: "cmd -l" },
  },
});

const hostHelper = (about: string) =>
  ({
    kind: "text",
    prompt: "Host",
    helper: {
      label: "Host",
      about,
      example: "1.1.1.1",
      find: { label: "Show the TV's address", command: "ip -o addr show" },
    },
  }) as Step;

const ip: Step = words("Network addresses and routes", [
  w("addr", "Addresses on each connection.", {
    next: words(
      "Which view",
      [
        w("show", "For a single connection.", {
          next: list("Which connection", "interfaces"),
        }),
      ],
      { optional: true } as Partial<Step>,
    ),
  }),
  w("link", "Each network connection."),
  w("route", "Where traffic goes."),
  w("neigh", "Devices the TV has seen nearby."),
  w("-o addr show", "Addresses, one per line."),
  w("-4 addr show", "Only IPv4 addresses."),
  w("-6 addr show", "Only IPv6 addresses."),
]);

const procFiles: Word[] = [
  ["/proc/cpuinfo", "The processor."],
  ["/proc/meminfo", "Memory, in detail."],
  ["/proc/version", "The Linux version."],
  ["/proc/uptime", "Seconds since the last restart."],
  ["/proc/loadavg", "How busy the TV has been."],
  ["/proc/mounts", "Mounted storage."],
  ["/proc/partitions", "Storage partitions."],
  ["/proc/net/dev", "Traffic on each connection."],
  ["/proc/net/wireless", "Wi-Fi signal."],
].map(([word, about]) => w(word, about, { group: "System files" }));

const termuxFile = (more: Partial<Step> = {}) =>
  list("Which file", "termuxFiles", {
    other: {
      label: "File",
      about: "A path inside Termux. ~ is its home folder.",
      example: "~/backup.sh",
      find: {
        label: "List Termux's home folder",
        command: inTermux("ls -la ~"),
      },
    },
    ...more,
  } as Partial<Extract<Step, { kind: "list" }>>);
const termuxPackage = (prompt: string) =>
  text(prompt, "The name of a Termux package.", "python", {
    pattern: /^[\w.+-]+$/,
    find: {
      label: "List what's installed",
      command: inTermux("apt list --installed 2>/dev/null"),
    },
  });
const termuxInstalled = list("Which package", "termuxPackages", {
  other: {
    label: "Package",
    about: "The name of a Termux package.",
    example: "python",
    pattern: /^[\w.+-]+$/,
  },
} as Partial<Extract<Step, { kind: "list" }>>);
/** apt's own run can outlast the 30 seconds a command gets, so it works in the background. */
const inBackground = (command: string) =>
  `mkdir -p ~/.bridgecoil; nohup sh -c '${command}' > ~/.bridgecoil/pkg.log 2>&1 &`;
const termux: Step = words("Run inside Termux", [
  w("pkg list-installed", "Packages in Termux.", {
    as: "apt list --installed 2>/dev/null",
    group: "Packages",
  }),
  w("pkg list-all", "Every package you could install.", {
    as: "apt list 2>/dev/null",
    group: "Packages",
  }),
  w("pkg search", "Look for a package.", {
    as: "apt search",
    group: "Packages",
    next: text(
      "Search for",
      "A word from the package's name or description.",
      "python",
    ),
  }),
  w("pkg show", "Details of a package.", {
    as: "apt show",
    group: "Packages",
    next: termuxPackage("Package"),
  }),
  change("pkg install", "Install a package. It runs in the background.", {
    as: "pkg-install",
    group: "Packages",
    next: termuxPackage("Package"),
  }),
  danger(
    "pkg uninstall",
    "Remove a package.",
    "Bridgecoil's Termux setup needs bash and its core packages.",
    {
      as: "pkg-uninstall",
      group: "Packages",
      next: termuxInstalled,
    },
  ),
  change("pkg update", "Refresh the package list, in the background.", {
    as: inBackground("apt-get update"),
    group: "Packages",
  }),
  change("pkg upgrade", "Upgrade everything, in the background.", {
    as: inBackground("apt-get -y upgrade"),
    group: "Packages",
  }),
  change("pkg autoclean", "Delete downloaded package files.", {
    as: "apt-get autoclean",
    group: "Packages",
  }),
  w("Show the install log", "How the last background install went.", {
    as: "tail -n 40 ~/.bridgecoil/pkg.log",
    group: "Packages",
  }),
  w("termux-info", "Termux's own details.", { group: "About Termux" }),
  w("uname -a", "The Linux version.", { group: "About Termux" }),
  w("df -h", "Free space.", { group: "About Termux" }),
  w("du -sh ~", "How much Termux's home folder holds.", {
    group: "About Termux",
  }),
  w("ls -la ~", "Files in Termux's home folder.", {
    group: "Files and scripts",
  }),
  w("ls -la ~/.termux/boot", "Scripts that run after a restart.", {
    group: "Files and scripts",
  }),
  w("cat", "Show a file.", { group: "Files and scripts", next: termuxFile() }),
  change("bash", "Run a script.", {
    group: "Files and scripts",
    warning: "A script can do anything Termux can.",
    next: termuxFile(),
  }),
  change("chmod +x", "Let a script run on its own.", {
    group: "Files and scripts",
    next: termuxFile(),
  }),
  danger("rm", "Delete a file.", "The file is gone for good.", {
    group: "Files and scripts",
    next: termuxFile(),
  }),
]);

export const root: Step = words("Pick a command", [
  w("pm", "Apps: list, permissions, disable, uninstall.", {
    group: "Apps",
    next: pm,
  }),
  w("am", "Open and stop apps, send broadcasts.", { group: "Apps", next: am }),
  w("appops", "See and change what an app can access.", {
    group: "Apps",
    next: appops,
  }),
  change("monkey", "Send random input to an app, for testing.", {
    as: "monkey -p",
    group: "Apps",
    warning: "Random input can press anything inside that app.",
    next: app({
      next: words("How many events", [
        w("100", "A quick test.", { as: "-v 100" }),
        w("1000", "A longer test.", { as: "-v 1000" }),
      ]),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  w("settings", "Read and change hidden settings.", {
    group: "Settings",
    next: settings,
  }),
  w("getprop", "Read a system property, or all of them.", {
    group: "Settings",
    next: list("Which property", "properties", {
      optional: true,
      other: {
        label: "Property",
        about: "A dotted name.",
        example: "ro.build.version.release",
        find: { label: "List every property", command: "getprop" },
      },
    }),
  }),
  change("setprop", "Change a system property.", {
    group: "Settings",
    warning: "Android only lets the shell change a few of these.",
    next: list("Which property", "properties", {
      other: {
        label: "Property",
        about: "A dotted name.",
        example: "debug.hwui.overdraw",
      },
      next: text("New value", "The value to store.", "1"),
    } as Partial<Extract<Step, { kind: "list" }>>),
  }),
  w("media", "Sound and playback.", {
    group: "Screen and input",
    next: words("What to do", [
      w("list-sessions", "Apps that are playing."),
      w("volume --stream 3 --get", "The media volume."),
      change("dispatch play-pause", "Play or pause."),
      change("dispatch next", "Next."),
      change("dispatch previous", "Previous."),
    ]),
  }),
  w("wm", "Screen size and text size.", {
    group: "Screen and input",
    next: wm,
  }),
  w("input", "Press keys, type text, tap the screen.", {
    group: "Screen and input",
    next: input,
  }),
  w("ime", "On-screen keyboards.", {
    group: "Screen and input",
    next: words("What to do", [
      w("list -s", "Installed keyboards."),
      change("set", "Choose the keyboard.", {
        next: list("Which keyboard", "keyboards"),
      }),
      change("enable", "Turn a keyboard on.", {
        next: list("Which keyboard", "keyboards"),
      }),
    ]),
  }),
  change("screencap", "Save a screenshot on the TV.", {
    as: "screencap -p",
    group: "Screen and input",
    next: text(
      "File to save",
      "Where to save it on the TV. Tools > Screen capture puts one on your phone.",
      "/sdcard/screen.png",
      {
        pattern: /^\/.+\.png$/,
      },
    ),
  }),
  change("screenrecord", "Record the screen to a file on the TV.", {
    group: "Screen and input",
    next: words(
      "How long",
      [
        w("10 seconds", "Short.", { as: "--time-limit 10" }),
        w("25 seconds", "The longest a command can run here.", {
          as: "--time-limit 25",
        }),
      ],
      {
        next: text(
          "File to save",
          "Where to save it on the TV. Tools > Screen capture puts one on your phone.",
          "/sdcard/screen.mp4",
          {
            pattern: /^\/.+\.mp4$/,
          },
        ),
      } as Partial<Step>,
    ),
  }),
  w("getevent", "See what the remote sends, for 8 seconds.", {
    as: "timeout 8 getevent -l || true",
    group: "Screen and input",
  }),
  w("getevent -il", "Remotes and keyboards the TV can see.", {
    group: "Screen and input",
    needs: "getevent",
  }),
  w("uiautomator dump", "Save a description of the screen.", {
    as: "uiautomator dump",
    effect: "change",
    group: "Screen and input",
    next: text(
      "File to save",
      "Where to save it on the TV.",
      "/sdcard/window.xml",
      {
        pattern: /^\/.+\.xml$/,
      },
    ),
  }),
  w("dumpsys", "Detailed reports from every part of Android.", {
    as: "timeout 25 dumpsys",
    group: "Find out what's going on",
    next: dumpsysStep,
  }),
  w("logcat", "The TV's log.", {
    group: "Find out what's going on",
    next: logcat,
  }),
  w("dmesg", "Messages from the Linux kernel.", {
    group: "Find out what's going on",
  }),
  w("ps", "Running programs.", {
    as: "ps -A",
    group: "Find out what's going on",
  }),
  w("top", "What's using the processor right now.", {
    as: "top -b -n 1 -m 15",
    group: "Find out what's going on",
  }),
  w("service list", "Android's system services.", {
    group: "Find out what's going on",
  }),
  w("cmd", "Ask a system service to do something.", {
    group: "Find out what's going on",
    next: cmd,
  }),
  w("pgrep", "Find a running program by name.", {
    as: "pgrep -l",
    group: "Find out what's going on",
    next: text("Name", "Part of the program's name.", "termux", {
      find: { label: "List running programs", command: "ps -A -o PID,NAME" },
    }),
  }),
  w("pidof", "Find a program's process number.", {
    group: "Find out what's going on",
    next: list("Which program", "packages", { other: appHelper }),
  }),
  w("uname -a", "The Linux version.", {
    group: "About the TV",
    needs: "uname",
  }),
  w("uptime", "How long the TV has been on.", { group: "About the TV" }),
  w("date", "The TV's date and time.", { group: "About the TV" }),
  w("id", "Which user commands run as.", { group: "About the TV" }),
  w("whoami", "The name of that user.", { group: "About the TV" }),
  w("hostname", "The TV's network name.", { group: "About the TV" }),
  w("getenforce", "Whether SELinux is enforcing.", { group: "About the TV" }),
  w("printenv", "The shell's environment.", { group: "About the TV" }),
  w("free -m", "Memory, in MB.", { group: "About the TV", needs: "free" }),
  w("vmstat", "Memory and processor activity.", { group: "About the TV" }),
  w("df -h", "Free space on every drive.", {
    group: "About the TV",
    needs: "df",
  }),
  w("lsusb", "USB devices.", { group: "About the TV" }),
  w("lsmod", "Loaded kernel modules.", { group: "About the TV" }),
  w("mount", "Mounted storage.", { group: "About the TV" }),
  w("sm", "Storage volumes.", {
    group: "About the TV",
    next: words("What to list", [
      w("list-disks", "Disks."),
      w("list-volumes", "Volumes."),
      w("get-primary-storage-uuid", "Where apps store things."),
      w("has-adoptable", "Whether a disk can be adopted."),
    ]),
  }),
  w("dpm list-owners", "Whether a device owner runs this TV.", {
    group: "About the TV",
    minSdk: 29,
  }),
  w("ip", "Addresses, connections and routes.", { group: "Network", next: ip }),
  w("ifconfig", "Network connections.", { group: "Network" }),
  w("ping", "Check whether something answers.", {
    as: "ping -c 4 -W 2",
    group: "Network",
    next: words("Where to check", [
      w("1.1.1.1", "Cloudflare. Tests the connection only."),
      w("8.8.8.8", "Google. Tests the connection only."),
      w("google.com", "Also tests that names can be looked up."),
      w("Another address", "Type your own.", {
        as: "",
        next: hostHelper("An IP address or a host name."),
      }),
    ]),
  }),
  w("traceroute", "See the route to somewhere.", {
    as: "timeout 25 traceroute -m 15 -w 1",
    group: "Network",
    next: hostHelper("An IP address or a host name."),
  }),
  w("nslookup", "Look up a name.", {
    group: "Network",
    next: hostHelper("A host name."),
  }),
  w("netstat -tuln", "Open ports.", { group: "Network", needs: "netstat" }),
  w("ss -tuln", "Open ports, newer tool.", { group: "Network", needs: "ss" }),
  w("svc", "Wi-Fi, Bluetooth and staying awake.", {
    group: "Network",
    next: svc,
  }),
  w("ls", "List a folder.", {
    as: "timeout 25 ls",
    group: "Files",
    next: flags(
      "Options",
      [
        w("-l", "Size, date and owner."),
        w("-a", "Include hidden files."),
        w("-h", "Sizes in KB and MB."),
        w("-t", "Newest first."),
        w("-S", "Biggest first."),
        w("-R", "Include folders inside."),
      ],
      folder("Which folder", { optional: true } as Partial<Step>),
    ),
  }),
  w("cat", "Show a file.", {
    group: "Files",
    next: words("What to show", [
      w("Pick a file", "Browse the TV's files.", {
        as: "",
        next: path("Which file"),
      }),
      ...procFiles,
    ]),
  }),
  w("head", "Show the first lines of a file.", {
    as: "head -n 40",
    group: "Files",
    next: path("Which file"),
  }),
  w("tail", "Show the last lines of a file.", {
    as: "tail -n 40",
    group: "Files",
    next: path("Which file"),
  }),
  w("grep", "Search a file for some text.", {
    as: "grep -i",
    group: "Files",
    next: text("Text to find", "Capitals don't matter.", "error", {}, {
      next: path("Which file"),
    } as Partial<Step>),
  }),
  w("find", "Find files by name.", {
    as: "timeout 25 find",
    group: "Files",
    next: folder("Where to look", {
      next: words("Find what", [
        w("-name", "Files with this name. * matches anything.", {
          next: text("Name", "* matches anything.", "*.apk"),
        }),
      ]),
    } as Partial<Step>),
  }),
  w("du -sh", "How much a folder holds.", {
    as: "timeout 25 du -sh",
    group: "Files",
    next: folder("Which folder"),
  }),
  w("stat", "A file's size, dates and owner.", {
    group: "Files",
    next: path("Which file"),
  }),
  w("file", "What kind of file something is.", {
    group: "Files",
    next: path("Which file"),
  }),
  w("wc -l", "Count a file's lines.", {
    group: "Files",
    next: path("Which file"),
  }),
  w("md5sum", "A file's MD5 checksum.", {
    group: "Files",
    next: path("Which file"),
  }),
  w("sha256sum", "A file's SHA-256 checksum.", {
    group: "Files",
    next: path("Which file"),
  }),
  w("sqlite3", "Read a database file.", {
    as: "sqlite3 -readonly",
    group: "Files",
    next: path("Which database", {
      next: words("What to read", [
        w(".tables", "List its tables."),
        w(".schema", "Show its structure."),
      ]),
    } as Partial<Step>),
  }),
  change("mkdir", "Make a folder.", {
    group: "Change files",
    next: text("New folder", "A full path.", "/sdcard/Backups", {
      pattern: /^\//,
    }),
  }),
  change("touch", "Make an empty file.", {
    group: "Change files",
    next: text("New file", "A full path.", "/sdcard/empty.txt", {
      pattern: /^\//,
    }),
  }),
  change("cp", "Copy a file.", {
    group: "Change files",
    next: path("What to copy", {
      next: folder("Where to put it"),
    } as Partial<Step>),
  }),
  change("cp -r", "Copy a folder.", {
    group: "Change files",
    next: folder("What to copy", {
      next: folder("Where to put it"),
    } as Partial<Step>),
  }),
  change("mv", "Move or rename.", {
    group: "Change files",
    next: path("What to move", {
      next: folder("Where to put it"),
    } as Partial<Step>),
  }),
  change("chmod", "Change who can use a file.", {
    group: "Change files",
    next: words(
      "Which permission",
      [
        w("+x", "Let it run as a program."),
        w("644", "Owner can change it, everyone can read it."),
        w("755", "Owner can change it, everyone can run it."),
      ],
      { next: path("Which file") } as Partial<Step>,
    ),
  }),
  change("tar -czf", "Pack a folder into one file.", {
    group: "Change files",
    next: text(
      "Archive to make",
      "A full path ending in .tar.gz.",
      "/sdcard/backup.tar.gz",
      { pattern: /^\/.+/ },
      {
        next: folder("What to pack"),
      } as Partial<Step>,
    ),
  }),
  danger("rm", "Delete a file.", "The file is gone for good.", {
    group: "Change files",
    next: path("Which file"),
  }),
  danger(
    "rm -r",
    "Delete a folder and everything in it.",
    "Everything inside is gone for good.",
    {
      group: "Change files",
      next: folder("Which folder"),
    },
  ),
  danger("rmdir", "Delete an empty folder.", "The folder is gone.", {
    group: "Change files",
    next: folder("Which folder"),
  }),
  change("kill", "End one of the shell's own programs.", {
    group: "Change files",
    next: list("Which program", "shellProcesses"),
  }),
  change("log", "Add a line to the TV's log.", {
    as: "log -t Bridgecoil",
    group: "Change files",
    next: text("Message", "The text to write.", "Started testing here"),
  }),
  change("sync", "Finish writing everything to storage.", {
    group: "Change files",
  }),
  w("termux", "Run something inside Termux on the TV.", {
    group: "Termux",
    as: "",
    needs: "termux",
    wrap: (inner) => inTermux(termuxCommand(inner)),
    next: termux,
  }),
  danger(
    "reboot",
    "Restart the TV.",
    "Anything playing stops, and you'll need to reconnect.",
    {
      group: "Restart",
      next: words(
        "How to restart",
        [
          danger(
            "-p",
            "Shut down instead.",
            "You'll need the remote to turn it back on.",
          ),
          danger(
            "recovery",
            "Into recovery mode.",
            "You'll need the remote or a keyboard to get out of recovery.",
          ),
          danger(
            "bootloader",
            "Into the bootloader.",
            "Many TVs have no way out of this without a keyboard or a power cut.",
          ),
        ],
        { optional: true } as Partial<Step>,
      ),
    },
  ),
]);

/** Turns the package words into what Termux runs, so `pkg install` survives the 30 second limit. */
function termuxCommand(inner: string) {
  const job = (verb: string) => inner.replace(`pkg-${verb} `, "").split(" ")[0];
  if (inner.startsWith("pkg-install "))
    return inBackground(`apt-get -y install ${job("install")}`);
  if (inner.startsWith("pkg-uninstall "))
    return inBackground(`apt-get -y remove ${job("uninstall")}`);
  return inner;
}
