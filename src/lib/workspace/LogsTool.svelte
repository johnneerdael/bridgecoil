<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import { targetOf, type LaunchableApp, type Platform } from "./client";
  import Output from "./Output.svelte";
  let { device, api }: { device: Device; api: Platform } = $props();
  const levels = [
    { id: "error", label: "Only errors" },
    { id: "warning", label: "Warnings and errors" },
    { id: "info", label: "Normal detail" },
    { id: "debug", label: "Everything" },
  ];
  const amounts = [200, 1000, 5000];
  let level = $state("warning");
  let app = $state("");
  let lines = $state(200);
  let apps = $state<{ package: string; label: string }[]>([]);
  let output = $state("");
  let shown = $state(false);
  let notRunning = $state(false);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  $effect(() => {
    const target = targetOf(device);
    let cancelled = false;
    void api
      .query<LaunchableApp[]>(target, { kind: "launchableApps" })
      .then((found) => {
        if (cancelled) return;
        const seen = new Set<string>();
        apps = found
          .map((a) => ({ package: a.component.split("/")[0], label: a.label }))
          .filter((a) => !seen.has(a.package) && !!seen.add(a.package));
      })
      // The list only narrows the logs, which still work without it.
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  });
  async function show() {
    busy = true;
    error = "";
    notice = "";
    try {
      const r = await api.query<{ output: string; notRunning: boolean }>(
        targetOf(device),
        { kind: "logs", level, package: app || null, lines },
      );
      output = r.output;
      notRunning = r.notRunning;
      shown = true;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function save() {
    error = "";
    notice = "";
    try {
      if (await api.saveFile("tv-log.txt", { text: output }))
        notice = "Log saved.";
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if notice}<p class="subtle" role="status">{notice}</p>{/if}
<div class="mapping-choice">
  <label for="log-level">Show</label>
  <select id="log-level" bind:value={level}>
    {#each levels as item}<option value={item.id}>{item.label}</option>{/each}
  </select>
  <label for="log-app">From</label>
  <select id="log-app" bind:value={app}>
    <option value="">The whole TV</option>
    {#each apps as item}<option value={item.package}>{item.label}</option
      >{/each}
  </select>
  <label for="log-lines">Newest</label>
  <select id="log-lines" bind:value={lines}>
    {#each amounts as amount}<option value={amount}>{amount} lines</option
      >{/each}
  </select>
</div>
<div class="capture-actions">
  <button class="primary" disabled={busy} onclick={show}
    >{busy ? "Reading…" : "Show logs"}</button
  >
  {#if output}<button class="secondary" onclick={save}>Save log</button>{/if}
</div>
{#if notRunning}<p class="subtle">
    That app isn't running. Open it on the TV, then try again.
  </p>
{:else if shown && !output}<p class="subtle">Nothing matches these choices.</p>
{:else if output}<Output text={output} />{/if}
