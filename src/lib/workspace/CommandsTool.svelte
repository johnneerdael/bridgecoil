<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type LaunchableApp,
    type Platform,
    type Review,
  } from "./client";
  import {
    buildCommand,
    catalog,
    defaults,
    findCommand,
    loadSaved,
    savedTitle,
    storeSaved,
    type Command,
    type CommandGroup,
    type CommandOption,
    type SavedCommand,
    type Values,
  } from "./commands";
  import Icon from "./Icon.svelte";
  let {
    device,
    api,
    onreview,
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
  } = $props();
  let group = $state<CommandGroup | null>(null);
  let command = $state<Command | null>(null);
  let values = $state<Values>({});
  let apps = $state<{ package: string; label: string }[]>([]);
  let busy = $state(false);
  let error = $state("");
  let saved = $state<SavedCommand[]>(loadSaved());
  const title = $derived(
    command
      ? savedTitle(
          command,
          values,
          apps.find((a) => a.package === values.app)?.label,
        )
      : "",
  );
  const isSaved = $derived(
    saved.some(
      (item) => item.id === command?.id && item.title === title,
    ),
  );
  function remember(next: SavedCommand[]) {
    saved = next;
    storeSaved(next);
  }
  function save() {
    if (!command || isSaved) return;
    remember([...saved, { id: command.id, title, values: { ...values } }]);
  }
  async function reopen(item: SavedCommand) {
    const found = findCommand(item.id);
    if (!found) return;
    group = found.group;
    await choose(found.command, item.values);
  }
  const effectLabels = {
    read: "Only reads",
    change: "Changes the TV",
    danger: "Be careful",
  };
  const needsApp = $derived(
    command?.options?.some((o) => o.kind === "app") ?? false,
  );
  const complete = $derived(!needsApp || !!values.app);
  const preview = $derived(
    command && complete ? buildCommand(command, values) : "",
  );
  function chosenAbout(option: CommandOption) {
    return option.kind === "choice"
      ? option.choices.find((c) => c.value === values[option.id])?.about
      : undefined;
  }
  async function choose(next: Command, kept?: Values) {
    command = next;
    values = { ...defaults(next), ...kept, profile: String(device.profileId) };
    error = "";
    if (!next.options?.some((o) => o.kind === "app")) return;
    try {
      if (!apps.length) {
        const found = await api.query<LaunchableApp[]>(targetOf(device), {
          kind: "launchableApps",
        });
        apps = found.map((a) => ({
          package: a.component.split("/")[0],
          label: a.label,
        }));
      }
      if (kept?.app && !apps.some((a) => a.package === kept.app)) {
        // Never swap in a different app: this might be about to delete its data.
        values = { ...values, app: "" };
        error = "That app isn't on this TV.";
      } else if (!values.app) values = { ...values, app: apps[0]?.package ?? "" };
    } catch (e) {
      error = errorMessage(e);
    }
  }
  async function run() {
    if (!preview) return;
    busy = true;
    error = "";
    try {
      onreview(
        await api.prepare(targetOf(device), {
          kind: "shell",
          command: preview,
        }),
      );
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if !group}
  <div class="row-list app-list">
    {#each saved as item (item.title)}<div class="file-row">
        <button class="list-row" onclick={() => reopen(item)}
          ><span class="truncate"
            ><span class="entry-name">{item.title}</span><small>Saved</small
            ></span
          ></button
        ><button
          class="icon-button file-more"
          aria-label={`Forget ${item.title}`}
          onclick={() => remember(saved.filter((s) => s !== item))}
          ><Icon name="close" /></button
        >
      </div>{/each}
    {#each catalog as item (item.name)}<button
        class="list-row"
        onclick={() => (group = item)}
        ><span class="truncate"
          ><span class="entry-name">{item.name}</span><small>{item.about}</small
          ></span
        ><Icon name="chevron-right" /></button
      >{/each}
  </div>
{:else if !command}
  <div class="toolbar">
    <button class="secondary" onclick={() => (group = null)}
      ><Icon name="arrow-left" />All commands</button
    >
  </div>
  <div class="row-list app-list">
    {#each group.commands as item (item.id)}<button
        class="list-row"
        onclick={() => choose(item)}
        ><span class="truncate"
          ><span class="entry-name">{item.title}</span><small>{item.about}</small
          ></span
        ><Icon name="chevron-right" /></button
      >{/each}
  </div>
{:else}
  <div class="toolbar">
    <button class="secondary" onclick={() => (command = null)}
      ><Icon name="arrow-left" />{group.name}</button
    >
  </div>
  <div class="setup-card">
    <h3>{command.title}</h3>
    <p>{command.about}</p>
    <p class="subtle">{effectLabels[command.effect]}</p>
    {#if command.warning}<p class="risk-note" role="note">
        {command.warning}
      </p>{/if}
    {#each command.options ?? [] as option (option.id)}
      <label for={`command-${option.id}`}>{option.label}</label>
      <select id={`command-${option.id}`} bind:value={values[option.id]}>
        {#if option.kind === "choice"}
          {#each option.choices as item}<option value={item.value}
              >{item.label}</option
            >{/each}
        {:else}
          {#each apps as item}<option value={item.package}>{item.label}</option
            >{/each}
        {/if}
      </select>
      <small class="subtle">{chosenAbout(option) ?? option.about}</small>
    {/each}
    {#if preview}
      <p class="subtle">This runs on the TV:</p>
      <code class="command-preview">{preview}</code>
    {/if}
    <button class="primary" disabled={busy || !preview} onclick={run}
      >Run on the TV</button
    >
    <button class="secondary" disabled={!preview || isSaved} onclick={save}
      >{isSaved ? "Saved" : "Save for later"}</button
    >
  </div>
{/if}
