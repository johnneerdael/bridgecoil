<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Platform,
    type Review,
    type Action,
    type Query,
    type Page,
  } from "./client";
  import Pager from "./Pager.svelte";
  import Icon from "./Icon.svelte";
  import Output from "./Output.svelte";
  let {
    device,
    api,
    onreview,
    oninstall,
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
    oninstall: () => void;
  } = $props();
  const tools = [
    {
      id: "launcher",
      name: "Home app",
      note: "Inspect and choose your launcher",
      icon: "home",
    },
    {
      id: "toolkit",
      name: "TV-side setup",
      note: "Shizuku, rish and Termux",
      icon: "shield",
    },
    { id: "files", name: "Files", note: "Browse TV storage", icon: "device" },
    {
      id: "diagnostics",
      name: "Diagnostics",
      note: "Logs, processes and device reports",
      icon: "info",
    },
    {
      id: "settings",
      name: "Settings",
      note: "Animations, brightness and audio",
      icon: "settings",
    },
    {
      id: "network",
      name: "Network",
      note: "Addresses and routing",
      icon: "wifi",
    },
    {
      id: "shell",
      name: "Shell",
      note: "Review an advanced command",
      icon: "terminal",
    },
  ];
  let page = $state(0);
  let tool = $state("");
  let busy = $state(false);
  let error = $state("");
  let output = $state("");
  let section = $state("logs");
  let command = $state("");
  let homes = $state<{ homes: string[]; current: string | null }>({
    homes: [],
    current: null,
  });
  type Tool = {
    name: string;
    package: string;
    installed: boolean;
    authorized: string;
    running: string;
    startup: string;
  };
  let toolkit = $state<Tool[]>([]);
  let setupPage = $state(0);
  type Setting = {
    namespace: string;
    key: string;
    name: string;
    value: string | null;
    error: string | null;
  };
  let settings = $state<Setting[]>([]);
  let settingPage = $state(0);
  let path = $state("/sdcard");
  let filePage = $state(0);
  let files = $state<Page<string>>({
    items: [],
    total: 0,
    page: 0,
    pageSize: 4,
  });
  let reload = $state(0);
  $effect(() => {
    device.id;
    tool = "";
    output = "";
    error = "";
  });
  $effect(() => {
    const target = targetOf(device),
      choice = tool,
      diagnostic = section,
      currentPath = path,
      p = filePage;
    reload;
    if (!choice || choice === "shell") return;
    let cancelled = false;
    busy = true;
    error = "";
    output = "";
    const q: Query =
      choice === "launcher"
        ? { kind: "launchers" }
        : choice === "toolkit"
          ? { kind: "toolkit" }
          : choice === "diagnostics"
            ? { kind: "diagnostics", section: diagnostic }
            : choice === "settings"
              ? { kind: "settings" }
              : choice === "files"
                ? { kind: "files", path: currentPath, page: p, pageSize: 3 }
                : { kind: "network" };
    void (async () => {
      try {
        if (choice === "launcher") {
          const r = await api.query<typeof homes>(target, q);
          if (!cancelled) homes = r;
        } else if (choice === "toolkit") {
          const r = await api.query<Tool[]>(target, q);
          if (!cancelled) toolkit = r;
        } else if (choice === "settings") {
          const r = await api.query<Setting[]>(target, q);
          if (!cancelled) settings = r;
        } else if (choice === "files") {
          const r = await api.query<{ path: string; entries: Page<string> }>(
            target,
            q,
          );
          if (!cancelled) files = r.entries;
        } else {
          const r = await api.query<{ output: string; forwarding?: string }>(
            target,
            q,
          );
          if (!cancelled)
            output = r.output + (r.forwarding ? "\n\n" + r.forwarding : "");
        }
      } catch (e) {
        if (!cancelled) error = errorMessage(e);
      } finally {
        if (!cancelled) busy = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });
  async function plan(action: Action) {
    if (busy) return;
    busy = true;
    error = "";
    try {
      onreview(await api.prepare(targetOf(device), action));
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function launch(component: string) {
    busy = true;
    error = "";
    try {
      const r = await api.remote(targetOf(device), {
        kind: "launch",
        component,
      });
      output =
        r.error ??
        `Launch ${r.status}. Check the TV before changing its default Home.`;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="feature-page">
  {#if !tool}<div class="tool-grid catalog">
      {#each tools.slice(page * 4, page * 4 + 4) as item}<button
          class="tool-tile"
          onclick={() => (tool = item.id)}
          ><Icon name={item.icon} /><strong>{item.name}</strong><small
            >{item.note}</small
          ></button
        >{/each}
    </div>
    <Pager {page} total={tools.length} size={4} onpage={(n) => (page = n)} />
  {:else}<div class="toolbar">
      <button class="secondary" onclick={() => (tool = "")}
        ><Icon name="arrow-left" />All tools</button
      ><strong>{tools.find((t) => t.id === tool)?.name}</strong><button
        class="icon-button"
        aria-label="Refresh tool"
        disabled={busy}
        onclick={() => reload++}><Icon name="refresh" /></button
      >
    </div>
    {#if busy}<p class="subtle" role="status">
        Inspecting TV…
      </p>{/if}{#if error}<p class="error-text" role="alert">{error}</p>{/if}
    {#if tool === "launcher"}<p class="subtle">
        Current Home: {homes.current ?? "Not reported"}. Test a replacement on
        the TV before requesting its default. Stock Home is kept enabled.
      </p>
      <div class="row-list">
        {#each homes.homes.slice(0, 4) as component}<div class="launcher-row">
            <span class="truncate">{component}</span><button
              class="secondary"
              disabled={busy}
              onclick={() => launch(component)}>Test</button
            ><button
              class="secondary"
              disabled={busy}
              onclick={() => plan({ kind: "defaultHome", component })}
              >Set Home</button
            >
          </div>{/each}
      </div>
      {#if output}<p class="subtle" role="status">{output}</p>{/if}
    {:else if tool === "toolkit"}
      {#if setupPage === 0}<div class="row-list">
          {#each toolkit as item}<div class="list-row">
              <span
                ><strong>{item.name}</strong><small>{item.package}</small></span
              ><span>{item.installed ? "Installed" : "Missing"}</span>
            </div>{/each}
        </div>
        <p class="subtle">
          Installation alone does not prove authorization, a running shell,
          startup configuration or reboot persistence.
        </p>
        <button class="secondary" onclick={oninstall}>Choose a tool APK</button>
      {:else if setupPage === 1}<h3>Authorize on the TV</h3>
        <p>
          Open Shizuku on the TV and follow its wireless debugging setup.
          Approve its own Android prompts. In Termux, use the rish files
          exported by that Shizuku installation and set the requested
          application ID.
        </p>
        <p class="subtle">
          Ordinary Termux and a Shizuku shell have different privileges.
          Existing Termux add-ons must share a compatible signing source; keep
          the TV's existing installation and data.
        </p>
      {:else}<h3>Startup is a separate check</h3>
        <p>
          Termux:Boot can start a configured shell listener after boot.
          First-run permission, authorization, the TV's network state and the
          listener must each be checked. A wake from standby is not a cold boot.
        </p>
        <p class="subtle">
          Authorization, running state, phone independence and cold-boot
          persistence remain unverified until their respective TV tests pass.
        </p>{/if}<Pager
        page={setupPage}
        total={3}
        size={1}
        onpage={(n) => (setupPage = n)}
      />
    {:else if tool === "files"}<form
        class="search-form"
        onsubmit={(e) => {
          e.preventDefault();
          filePage = 0;
          reload++;
        }}
      >
        <label class="sr-only" for="remote-path">Device path</label><input
          id="remote-path"
          bind:value={path}
        /><button class="secondary" type="submit" disabled={busy}>Open</button>
      </form>
      <div class="row-list">
        <button
          class="list-row"
          onclick={() => {
            path = path.split("/").slice(0, -1).join("/") || "/";
            filePage = 0;
          }}>Parent directory</button
        >{#each files.items.slice(0, 3) as entry}<button
            class="list-row"
            onclick={() => {
              path = entry;
              filePage = 0;
            }}
            ><span class="truncate">{entry.split("/").pop()}</span><Icon
              name="chevron-right"
            /></button
          >{/each}
      </div>
      <Pager
        page={filePage}
        total={files.total}
        size={3}
        {busy}
        onpage={(n) => (filePage = n)}
      />
    {:else if tool === "settings"}<div class="row-list">
        {#each settings.slice(settingPage * 2, settingPage * 2 + 2) as setting}<div
            class="setting-row"
          >
            <label for={`setting-${setting.key}`}
              >{setting.name}<small
                >Current: {setting.value ?? "Unknown"}{setting.namespace ===
                "global"
                  ? " · TV-wide"
                  : ""}</small
              ></label
            ><select
              id={`setting-${setting.key}`}
              disabled={busy || setting.value === null}
              value={setting.value ?? ""}
              onchange={(e) =>
                plan({
                  kind: "setting",
                  namespace: setting.namespace,
                  key: setting.key,
                  value: e.currentTarget.value,
                })}
              >{#each setting.key.endsWith("_scale") ? ["0", "0.5", "1", "1.5", "2", "5", "10"] : setting.key === "volume_music" ? ["0", "3", "5", "8", "10", "15"] : ["0", "32", "64", "128", "192", "255"] as value}<option
                  {value}>{value}</option
                >{/each}</select
            >
          </div>{/each}
      </div>
      <p class="subtle">
        Changes are reviewed before applying and read back afterwards. Firmware
        can override stored values.
      </p>
      <Pager
        page={settingPage}
        total={settings.length}
        size={2}
        {busy}
        onpage={(n) => (settingPage = n)}
      />
    {:else if tool === "shell"}<p class="subtle">
        Advanced · ADB shell on {device.model}, profile {device.profileId}.
        Commands may affect the whole TV. Output is bounded and recorded
        locally.
      </p>
      <form
        class="shell-form"
        onsubmit={(e) => {
          e.preventDefault();
          void plan({ kind: "shell", command });
        }}
      >
        <label for="shell-command">Command</label><textarea
          id="shell-command"
          bind:value={command}
          maxlength={8192}
          spellcheck={false}></textarea><button
          class="primary"
          disabled={busy || !command.trim()}>Review command</button
        >
      </form>
    {:else}{#if tool === "diagnostics"}<label
          class="sr-only"
          for="diagnostics-section">Diagnostics section</label
        ><select id="diagnostics-section" bind:value={section}
          >{#each [["logs", "Recent logs"], ["processes", "Processes"], ["memory", "Memory"], ["services", "Services"], ["display", "Display"], ["input", "Input devices"], ["battery", "Battery"]] as [value, label]}<option
              {value}>{label}</option
            >{/each}</select
        >{/if}<Output text={output} />{/if}
  {/if}
</section>
