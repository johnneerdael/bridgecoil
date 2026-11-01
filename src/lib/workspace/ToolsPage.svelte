<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Platform,
    type ButtonFollowUp,
    type Review,
    type Action,
    type BulkReview,
    type Query,
  } from "./client";
  import ButtonsTool from "./ButtonsTool.svelte";
  import CleanupTool from "./CleanupTool.svelte";
  import CommandsTool from "./CommandsTool.svelte";
  import FilesTool from "./FilesTool.svelte";
  import LauncherTool from "./LauncherTool.svelte";
  import LogsTool from "./LogsTool.svelte";
  import ScreenshotTool from "./ScreenshotTool.svelte";
  import SetupTool from "./SetupTool.svelte";
  import { internalStorage } from "./files";
  import Pager from "./Pager.svelte";
  import Icon from "./Icon.svelte";
  import { settingChoices, settingLabel, settingWritable } from "./settings";
  import Output from "./Output.svelte";
  let {
    device,
    api,
    onreview,
    onbulk,
    oninstall,
    tool = $bindable(""),
    buttonFollowUp = $bindable(null),
    filePath = $bindable(internalStorage),
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
    buttonFollowUp?: ButtonFollowUp | null;
    onbulk: (r: BulkReview) => void;
    oninstall: () => void;
    tool?: string;
    filePath?: string;
  } = $props();
  const tools = [
    {
      id: "toolkit",
      name: "TV setup",
      icon: "shield",
    },
    {
      id: "buttons",
      name: "Buttons",
      icon: "remap",
    },
    {
      id: "cleanup",
      name: "Clean up",
      icon: "check",
    },
    {
      id: "commands",
      name: "Commands",
      icon: "terminal",
    },
    {
      id: "launcher",
      name: "Home app",
      icon: "home",
    },
    {
      id: "capture",
      name: "Screen capture",
      icon: "device",
    },
    { id: "files", name: "Files", icon: "device" },
    {
      id: "diagnostics",
      name: "Diagnostics",
      icon: "info",
    },
    {
      id: "settings",
      name: "Settings",
      icon: "settings",
    },
    {
      id: "network",
      name: "Network",
      icon: "wifi",
    },
    {
      id: "shell",
      name: "Shell",
      icon: "terminal",
    },
  ];
  let busy = $state(false);
  let error = $state("");
  let output = $state("");
  let section = $state("logs");
  let command = $state("");
  type Setting = {
    namespace: string;
    key: string;
    name: string;
    value: string | null;
    error: string | null;
  };
  let settings = $state<Setting[]>([]);
  let settingPage = $state(0);
  let reload = $state(0);
  const selfLoading = [
    "shell",
    "files",
    "toolkit",
    "buttons",
    "cleanup",
    "commands",
    "launcher",
    "capture",
  ];
  $effect(() => {
    const target = targetOf(device),
      choice = tool,
      diagnostic = section;
    reload;
    // A load this replaces can no longer clear its own busy state.
    if (
      !choice ||
      selfLoading.includes(choice) ||
      (choice === "diagnostics" && diagnostic === "logs")
    ) {
      busy = false;
      return;
    }
    let cancelled = false;
    busy = true;
    error = "";
    output = "";
    const q: Query =
      choice === "launcher"
        ? { kind: "launchers" }
          : choice === "diagnostics"
            ? { kind: "diagnostics", section: diagnostic }
            : choice === "settings"
              ? { kind: "settings" }
              : { kind: "network" };
    void (async () => {
      try {
        if (choice === "settings") {
          const r = await api.query<Setting[]>(target, q);
          if (!cancelled) settings = r;
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
</script>

<section class="feature-page">
  {#if !tool}<div class="tool-grid catalog">
      {#each tools as item}<button
          class="tool-tile"
          onclick={() => (tool = item.id)}
          ><Icon name={item.icon} /><strong>{item.name}</strong></button
        >{/each}
    </div>
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
    {#if tool === "launcher"}<LauncherTool {device} {api} {reload} {onreview} {oninstall} />
    {:else if tool === "capture"}<ScreenshotTool {device} {api} />
    {:else if tool === "toolkit"}<SetupTool
        {device}
        {api}
        {reload}
        {onreview}
        onbuttons={() => (tool = "buttons")}
      />
    {:else if tool === "buttons"}<ButtonsTool
        {device}
        {api}
        {reload}
        {onreview}
        onsetup={() => (tool = "toolkit")}
        followUp={buttonFollowUp}
        onfollowup={(next) => (buttonFollowUp = next)}
      />
    {:else if tool === "cleanup"}<CleanupTool
        {device}
        {api}
        {reload}
        {onbulk}
      />
    {:else if tool === "commands"}<CommandsTool {device} {api} {onreview} />
    {:else if tool === "files"}<FilesTool
        {device}
        {api}
        {reload}
        {onreview}
        bind:path={filePath}
      />
    {:else if tool === "settings"}<div class="row-list">
        {#each settings.slice(settingPage * 2, settingPage * 2 + 2) as setting}{@const choices =
            settingChoices(setting.key, setting.value)}<div
            class="setting-row"
          >
            <label for={`setting-${setting.key}`}
              >{setting.name}<small
                >Current: {settingLabel(setting.value)}{setting.namespace ===
                "global"
                  ? " · TV-wide"
                  : ""}</small
              ></label
            ><select
              id={`setting-${setting.key}`}
              disabled={busy || setting.value === null}
              value={choices.selected}
              onchange={(e) =>
                plan({
                  kind: "setting",
                  namespace: setting.namespace,
                  key: setting.key,
                  value: e.currentTarget.value,
                })}
              >{#each choices.options as value}<option
                  {value}
                  hidden={!settingWritable(value)}>{settingLabel(value)}</option
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
        >{/if}{#if tool === "diagnostics" && section === "logs"}<LogsTool
          {device}
          {api}
        />{:else}<Output text={output} />{/if}{/if}
  {/if}
</section>
