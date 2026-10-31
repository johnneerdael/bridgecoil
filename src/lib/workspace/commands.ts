export type Effect = "read" | "change" | "danger";
export type Choice = { value: string; label: string; about: string };
export type CommandOption =
  | {
      id: string;
      kind: "choice";
      label: string;
      about: string;
      choices: Choice[];
    }
  | { id: "app"; kind: "app"; label: string; about: string };
export type Values = Record<string, string>;
export type Command = {
  id: string;
  title: string;
  about: string;
  effect: Effect;
  warning?: string;
  options?: CommandOption[];
  build: (values: Values) => string;
};
export type CommandGroup = { name: string; about: string; commands: Command[] };

export function shellQuote(value: string) {
  return /^[\w@%+=:,./-]+$/.test(value)
    ? value
    : `'${value.replaceAll("'", `'"'"'`)}'`;
}

const app = (about: string): CommandOption => ({
  id: "app",
  kind: "app",
  label: "App",
  about,
});
const choice = (
  id: string,
  label: string,
  about: string,
  choices: [string, string, string][],
): CommandOption => ({
  id,
  kind: "choice",
  label,
  about,
  choices: choices.map(([value, name, detail]) => ({
    value,
    label: name,
    about: detail,
  })),
});
const read = (
  id: string,
  title: string,
  about: string,
  command: string,
): Command => ({ id, title, about, effect: "read", build: () => command });
const q = shellQuote;
const onOff: [string, string, string][] = [
  ["1", "On", "Turns this on."],
  ["0", "Off", "Turns this off."],
];

export const catalog: CommandGroup[] = [
  {
    name: "Apps",
    about: "See, stop, disable and remove apps",
    commands: [
      {
        id: "list-apps",
        title: "List apps",
        about: "Shows the package name of every app in the group you pick.",
        effect: "read",
        options: [
          choice("group", "Which apps", "Narrows down the list.", [
            ["", "All", "Every app on the TV."],
            ["-3", "Installed by you", "Apps that didn't come with the TV."],
            ["-s", "Preinstalled", "Apps that came with the TV."],
            ["-d", "Disabled", "Apps that are switched off."],
          ]),
        ],
        build: (v) => `pm list packages ${v.group}`.trim(),
      },
      {
        id: "app-version",
        title: "Show an app's version",
        about: "Reads the installed version of an app.",
        effect: "read",
        options: [app("The app to look at.")],
        build: (v) =>
          `dumpsys package ${q(v.app)} | grep -m2 -E 'versionName|versionCode'`,
      },
      {
        id: "app-permissions",
        title: "Show an app's permissions",
        about: "Lists what an app has asked for and whether it was granted.",
        effect: "read",
        options: [app("The app to look at.")],
        build: (v) => `dumpsys package ${q(v.app)} | grep -E 'permission\\.'`,
      },
      {
        id: "app-location",
        title: "Find an app's files",
        about: "Shows where the app is installed on the TV.",
        effect: "read",
        options: [app("The app to look at.")],
        build: (v) => `pm path ${q(v.app)}`,
      },
      {
        id: "force-stop",
        title: "Force stop an app",
        about: "Closes the app now. It can start again later.",
        effect: "change",
        options: [app("The app to close.")],
        build: (v) => `am force-stop ${q(v.app)}`,
      },
      {
        id: "disable-app",
        title: "Switch an app off",
        about: "Hides the app and stops it running. Its data stays on the TV.",
        effect: "danger",
        warning:
          "Switching off Settings or your launcher can leave you unable to get back in without this app.",
        options: [app("The app to switch off.")],
        build: (v) => `pm disable-user --user ${v.profile} ${q(v.app)}`,
      },
      {
        id: "enable-app",
        title: "Switch an app back on",
        about: "Reverses switching an app off.",
        effect: "change",
        options: [app("The app to switch on.")],
        build: (v) => `pm enable --user ${v.profile} ${q(v.app)}`,
      },
      {
        id: "clear-app",
        title: "Delete an app's data",
        about: "Resets the app as if it was just installed.",
        effect: "danger",
        warning: "Sign-ins, settings and downloads in this app are deleted.",
        options: [app("The app to reset.")],
        build: (v) => `pm clear --user ${v.profile} ${q(v.app)}`,
      },
      {
        id: "uninstall-app",
        title: "Remove an app",
        about: "Removes the app for this profile.",
        effect: "danger",
        warning:
          "Its data is deleted. Removing a system app can break the TV, so check Apps for its rating first.",
        options: [app("The app to remove.")],
        build: (v) => `pm uninstall --user ${v.profile} ${q(v.app)}`,
      },
    ],
  },
  {
    name: "Display",
    about: "Screen size, text size and animations",
    commands: [
      read("screen-size", "Show the screen size", "Reads the resolution the TV uses.", "wm size"),
      read("screen-density", "Show the text and icon size", "Reads the current screen density.", "wm density"),
      {
        id: "set-density",
        title: "Change text and icon size",
        about: "Makes everything on screen bigger or smaller.",
        effect: "change",
        options: [
          choice("density", "Screen density", "Higher numbers look bigger.", [
            ["160", "160", "Smaller text. Fits more on screen."],
            ["213", "213", "The usual size for many TVs."],
            ["240", "240", "Larger text, easier to read from the sofa."],
            ["320", "320", "Very large text. Some apps may look wrong."],
          ]),
        ],
        build: (v) => `wm density ${v.density}`,
      },
      {
        id: "reset-density",
        title: "Reset text and icon size",
        about: "Goes back to the size the TV came with.",
        effect: "change",
        build: () => "wm density reset",
      },
      {
        id: "animation-speed",
        title: "Change animation speed",
        about: "Makes menus feel faster on a slow TV.",
        effect: "change",
        options: [
          choice("speed", "Speed", "Lower is faster.", [
            ["0", "Off", "No animations. The fastest feel."],
            ["0.5", "Fast", "Half the normal time."],
            ["1", "Normal", "The TV's default."],
          ]),
        ],
        build: (v) =>
          ["window_animation_scale", "transition_animation_scale", "animator_duration_scale"]
            .map((key) => `settings put global ${key} ${v.speed}`)
            .join("; "),
      },
      {
        id: "screensaver",
        title: "Turn the screensaver on or off",
        about: "Controls whether the TV shows a screensaver when idle.",
        effect: "change",
        options: [choice("state", "Screensaver", "Some TVs ignore this.", onOff)],
        build: (v) => `settings put secure screensaver_enabled ${v.state}`,
      },
    ],
  },
  {
    name: "Input",
    about: "Press keys and inspect input devices",
    commands: [
      {
        id: "press-key",
        title: "Press a key",
        about: "Sends a key to the TV as if you pressed it on the remote.",
        effect: "change",
        options: [
          choice("key", "Key", "The key to send.", [
            ["KEYCODE_HOME", "Home", "Goes to the home screen."],
            ["KEYCODE_BACK", "Back", "Goes back one screen."],
            ["KEYCODE_MENU", "Menu", "Opens the menu in many apps."],
            ["KEYCODE_SETTINGS", "Settings", "Opens the TV's settings."],
            ["KEYCODE_TV_INPUT", "Change input", "Shows the HDMI input list."],
            ["KEYCODE_MEDIA_PLAY_PAUSE", "Play or pause", "Controls what's playing."],
            ["KEYCODE_VOLUME_MUTE", "Mute", "Mutes the sound."],
          ]),
        ],
        build: (v) => `input keyevent ${v.key}`,
      },
      read("input-devices", "List input devices", "Shows every remote, keyboard and controller the TV can see.", "getevent -il | grep -E 'add device|name:'"),
      read("keyboards", "List keyboards", "Shows the on-screen keyboards installed.", "ime list -s"),
    ],
  },
  {
    name: "Network",
    about: "Addresses, Wi-Fi and private DNS",
    commands: [
      read("ip-address", "Show network addresses", "Lists the TV's addresses on each connection.", "ip -o addr show"),
      read("wifi-status", "Show Wi-Fi status", "Shows the network and signal the TV is using.", "dumpsys wifi | grep -m1 mWifiInfo"),
      read("dns-mode", "Show private DNS", "Shows whether the TV uses private DNS.", "settings get global private_dns_mode; settings get global private_dns_specifier"),
      {
        id: "ping",
        title: "Test the internet connection",
        about: "Sends a few packets to see whether the TV can reach the internet.",
        effect: "read",
        options: [
          choice("host", "Check against", "Where to send the test.", [
            ["1.1.1.1", "Cloudflare", "A reliable public address. Tests the connection only."],
            ["8.8.8.8", "Google", "Another reliable public address."],
            ["google.com", "A website name", "Also tests that names can be looked up."],
          ]),
        ],
        build: (v) => `ping -c 3 ${v.host}`,
      },
      {
        id: "private-dns",
        title: "Block ads with private DNS",
        about: "Sends the TV's name lookups to a filtering service.",
        effect: "change",
        options: [
          choice("provider", "Service", "Pick one, or turn it off.", [
            ["dns.adguard-dns.com", "AdGuard", "Blocks ads and trackers."],
            ["security.cloudflare-dns.com", "Cloudflare", "Blocks malware."],
            ["off", "Off", "Goes back to your network's normal DNS."],
          ]),
        ],
        build: (v) =>
          v.provider === "off"
            ? "settings put global private_dns_mode off"
            : `settings put global private_dns_mode hostname; settings put global private_dns_specifier ${v.provider}`,
      },
      {
        id: "wifi-power",
        title: "Turn Wi-Fi on or off",
        about: "Switches the TV's Wi-Fi.",
        effect: "danger",
        warning:
          "Turning Wi-Fi off can disconnect this phone from the TV. You may need the remote to turn it back on.",
        options: [
          choice("state", "Wi-Fi", "Pick what you want.", [
            ["enable", "On", "Turns Wi-Fi on."],
            ["disable", "Off", "Turns Wi-Fi off."],
          ]),
        ],
        build: (v) => `svc wifi ${v.state}`,
      },
    ],
  },
  {
    name: "Power",
    about: "Restart, sleep and stay awake",
    commands: [
      {
        id: "sleep",
        title: "Put the TV to sleep",
        about: "Turns the screen off, like pressing Power.",
        effect: "change",
        build: () => "input keyevent KEYCODE_SLEEP",
      },
      {
        id: "stay-awake",
        title: "Keep the TV awake",
        about: "Stops the TV from going to sleep on its own.",
        effect: "change",
        options: [
          choice("state", "Stay awake", "Uses more power when on.", [
            ["true", "On", "The TV stays on until you turn it off."],
            ["false", "Off", "The TV sleeps as normal."],
          ]),
        ],
        build: (v) => `svc power stayon ${v.state}`,
      },
      {
        id: "reboot",
        title: "Restart the TV",
        about: "Restarts the TV. This disconnects Bridgecoil.",
        effect: "danger",
        warning: "Anything playing stops, and you'll need to reconnect.",
        build: () => "reboot",
      },
      {
        id: "shutdown",
        title: "Shut down the TV",
        about: "Turns the TV fully off.",
        effect: "danger",
        warning: "You'll need the remote or the TV's button to turn it back on.",
        build: () => "reboot -p",
      },
    ],
  },
  {
    name: "Settings",
    about: "Read and change hidden settings",
    commands: [
      {
        id: "list-settings",
        title: "List settings",
        about: "Shows every value stored in a group of settings.",
        effect: "read",
        options: [
          choice("group", "Group", "Android keeps settings in three groups.", [
            ["system", "System", "Everyday settings for this profile."],
            ["secure", "Secure", "Sensitive settings for this profile."],
            ["global", "Global", "Settings shared by the whole TV."],
          ]),
        ],
        build: (v) => `settings list ${v.group}`,
      },
      {
        id: "auto-updates",
        title: "Stop automatic system updates",
        about: "Asks the TV not to install system updates by itself.",
        effect: "change",
        options: [
          choice("state", "Automatic updates", "Some TVs ignore this.", [
            ["1", "Stop them", "Updates wait until you start them."],
            ["0", "Allow them", "The TV's normal behaviour."],
          ]),
        ],
        build: (v) => `settings put global ota_disable_automatic_update ${v.state}`,
      },
      {
        id: "show-touches",
        title: "Show touches for debugging",
        about: "Shows a dot wherever input lands on screen.",
        effect: "change",
        options: [choice("state", "Show touches", "Useful with a mouse.", onOff)],
        build: (v) => `settings put system show_touches ${v.state}`,
      },
    ],
  },
  {
    name: "System",
    about: "Model, storage, memory and health",
    commands: [
      read("uptime", "How long the TV has been on", "Shows the time since it last restarted.", "uptime"),
      read("android-version", "Show the Android version", "Reads the system version.", "getprop ro.build.version.release"),
      read("model", "Show the model", "Reads the name the maker gave this TV.", "getprop ro.product.model"),
      read("cpu", "Show the processor type", "Reads which app builds this TV can run.", "getprop ro.product.cpu.abilist"),
      read("storage", "Show free storage", "Shows how much space is left.", "df -h /data"),
      read("memory", "Show memory use", "Shows how much memory is free.", "head -n 5 /proc/meminfo"),
      read("top-apps", "Show what's using the processor", "Lists the busiest processes right now.", "top -b -n 1 -m 12"),
      read("temperature", "Show temperature", "Shows thermal status, where the TV reports it.", "dumpsys thermalservice | head -n 30"),
    ],
  },
  {
    name: "Logs",
    about: "Find out why something went wrong",
    commands: [
      {
        id: "recent-logs",
        title: "Read recent logs",
        about: "Shows the TV's recent system messages.",
        effect: "read",
        options: [
          choice("level", "Show", "Lower levels show more.", [
            ["*:E", "Errors only", "Only things that went wrong."],
            ["*:W", "Warnings and errors", "Also shows things that nearly went wrong."],
            ["*:I", "Everything useful", "A lot more detail."],
          ]),
          choice("count", "How many lines", "More lines take longer to read.", [
            ["100", "100", "A quick look."],
            ["500", "500", "A longer look back."],
          ]),
        ],
        build: (v) => `logcat -d -t ${v.count} ${q(v.level)}`,
      },
      read("crash-logs", "Read crash reports", "Shows recent app crashes.", "logcat -d -b crash -t 100"),
      read("boot-reason", "Why the TV last restarted", "Shows the reason the TV gives.", "getprop sys.boot.reason"),
    ],
  },
];

export function defaults(command: Command): Values {
  const values: Values = { profile: "0" };
  for (const option of command.options ?? [])
    values[option.id] = option.kind === "choice" ? option.choices[0].value : "";
  return values;
}
export const buildCommand = (command: Command, values: Values) =>
  command.build(values);

export type SavedCommand = {
  id: string;
  title: string;
  values: Values;
};
const savedKey = "bridgecoil.savedCommands";

/** Commands the user keeps close. A saved entry whose command no longer exists is dropped. */
export function loadSaved(): SavedCommand[] {
  try {
    const stored = JSON.parse(localStorage.getItem(savedKey) ?? "[]");
    return Array.isArray(stored)
      ? stored.filter((item: SavedCommand) => !!findCommand(item.id))
      : [];
  } catch {
    return [];
  }
}
export function storeSaved(saved: SavedCommand[]) {
  try {
    localStorage.setItem(savedKey, JSON.stringify(saved));
  } catch {
    // A convenience only: the command still runs, it just isn't remembered.
  }
}
export function findCommand(id: string) {
  for (const group of catalog) {
    const command = group.commands.find((item) => item.id === id);
    if (command) return { group, command };
  }
  return undefined;
}
/** Says which choices were made, so two saved versions of a command can be told apart. */
export function savedTitle(command: Command, values: Values, appLabel?: string) {
  const chosen = (command.options ?? []).map((option) =>
    option.kind === "choice"
      ? option.choices.find((c) => c.value === values[option.id])?.label
      : appLabel,
  );
  return [command.title, ...chosen.filter(Boolean)].join(" · ");
}
