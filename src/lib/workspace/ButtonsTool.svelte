<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type KeyIdentity,
    type LaunchableApp,
    type Mapping,
    type MappingChange,
    type MappingState,
    type Platform,
    type Review,
    type ToolkitSnapshot,
  } from "./client";
  import Icon from "./Icon.svelte";
  let {
    device,
    api,
    reload,
    onreview,
    onsetup,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    onsetup: () => void;
  } = $props();
  const listenSeconds = 8;
  let tv = $state<MappingState | null>(null);
  let toolkit = $state<ToolkitSnapshot | null>(null);
  let refresh = $state(0);
  let busy = $state(false);
  let error = $state("");
  type Screen = "list" | "listen" | "choose" | "test";
  let view = $state<Screen>("list");
  let listened = $state(false);
  let presses = $state<KeyIdentity[]>([]);
  let chosen = $state<KeyIdentity | null>(null);
  let apps = $state<LaunchableApp[]>([]);
  let testResult = $state("");
  let testFrom = 0;
  let testLast: string | undefined;
  const sameKey = (a: KeyIdentity, b: KeyIdentity) =>
    a.source === b.source && a.key === b.key && a.scan === b.scan;
  const existing = $derived(
    chosen && tv
      ? tv.mappings.find((m) => sameKey(m.key, chosen!))
      : undefined,
  );
  const describeKey = (key: KeyIdentity) =>
    [key.key, key.scan && `code ${key.scan}`, key.source]
      .filter(Boolean)
      .join(" · ");
  const status = $derived(
    !tv
      ? ""
      : tv.paused
        ? "Paused. Buttons do their normal action only."
        : tv.running
          ? "Listening for buttons on the TV."
          : "The listener isn't running on the TV.",
  );
  $effect(() => {
    const target = targetOf(device);
    reload;
    refresh;
    let cancelled = false;
    void Promise.all([
      api.query<MappingState>(target, { kind: "mappings" }),
      api.query<ToolkitSnapshot>(target, { kind: "toolkit" }),
    ])
      .then(([mappings, kit]) => {
        if (cancelled) return;
        tv = mappings;
        toolkit = kit;
      })
      .catch((e) => {
        if (!cancelled) error = errorMessage(e);
      });
    return () => {
      cancelled = true;
    };
  });
  async function attempt(work: () => Promise<void>) {
    busy = true;
    error = "";
    try {
      await work();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const change = (next: MappingChange) =>
    attempt(async () => {
      onreview(
        await api.prepare(targetOf(device), {
          kind: "mappings",
          change: next,
        }),
      );
    });
  const listen = () =>
    attempt(async () => {
      listened = false;
      const r = await api.query<{ presses: KeyIdentity[] }>(targetOf(device), {
        kind: "captureKeys",
        seconds: listenSeconds,
      });
      presses = r.presses;
      listened = true;
    });
  const pick = (key: KeyIdentity) =>
    attempt(async () => {
      chosen = key;
      if (!apps.length)
        apps = await api.query<LaunchableApp[]>(targetOf(device), {
          kind: "launchableApps",
        });
      view = "choose";
    });
  function save(app: LaunchableApp) {
    if (!chosen || !tv) return;
    const key = chosen;
    const kept = tv.mappings.filter((m) => !sameKey(m.key, key));
    view = "list";
    return change({
      kind: "apply",
      mappings: [...kept, { key, component: app.component, label: app.label }],
    });
  }
  function remove(mapping: Mapping) {
    if (!tv) return;
    const kept = tv.mappings.filter((m) => m !== mapping);
    return change(
      kept.length
        ? { kind: "apply", mappings: kept }
        : { kind: "removeAll" },
    );
  }
  const startTest = () =>
    attempt(async () => {
      testResult = "";
      const fresh = await api.query<MappingState>(targetOf(device), {
        kind: "mappings",
      });
      testFrom = fresh.log.length;
      testLast = fresh.log.at(-1);
      view = "test";
    });
  const checkTest = () =>
    attempt(async () => {
      const fresh = await api.query<MappingState>(targetOf(device), {
        kind: "mappings",
      });
      tv = fresh;
      // The log keeps a rolling tail, so find where the test began rather than trusting a count.
      const seen = testLast ? fresh.log.lastIndexOf(testLast) : -1;
      const fresher = fresh.log.slice(seen >= 0 ? seen + 1 : testFrom);
      const newest = fresher
        .reverse()
        .map((line) => ({
          failed: line.includes(" failed "),
          mapping: fresh.mappings.find((m) => line.endsWith(` ${m.component}`)),
        }))
        .find((entry) => entry.mapping);
      testResult = !newest
        ? "The TV didn't see a press of a mapped button."
        : newest.failed
          ? `The TV saw the press but couldn't open ${newest.mapping!.label}.`
          : `It worked: ${newest.mapping!.label} opened.`;
    });
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if !tv}{#if !error}<p class="subtle" role="status">Checking the TV…</p>{/if}
{:else if view === "listen"}
  <div class="setup-card">
    <h3>Press the button</h3>
    <p>
      Tap Listen, then press the button on your remote once. It may do its
      normal action on the TV while Bridgecoil listens.
    </p>
    <button class="primary" disabled={busy} onclick={listen}
      >{busy ? `Listening for ${listenSeconds} seconds…` : "Listen"}</button
    >
    {#if listened && !presses.length}<p class="subtle">
        Nothing reached Bridgecoil. Try again and press firmly. Power, voice and
        some TV-brand buttons never reach apps, so they can't be remapped.
      </p>{/if}
    {#each presses as press}<button
        class="list-row"
        disabled={busy}
        onclick={() => pick(press)}
        ><span class="truncate">{describeKey(press)}</span><Icon
          name="chevron-right"
        /></button
      >{/each}
    <button class="secondary" onclick={() => (view = "list")}>Cancel</button>
  </div>
{:else if view === "choose"}
  <div class="toolbar">
    <button class="secondary" onclick={() => (view = "listen")}
      ><Icon name="arrow-left" />Back</button
    >
  </div>
  <p>Which app should this button open?</p>
  {#if existing}<p class="subtle">
      This button currently opens {existing.label}.
    </p>{/if}
  <div class="row-list app-list">
    {#each apps as app (app.component)}<button
        class="list-row"
        disabled={busy}
        onclick={() => save(app)}
        ><span class="truncate">{app.label}</span><Icon
          name="chevron-right"
        /></button
      >{/each}
  </div>
{:else if view === "test"}
  <div class="setup-card">
    <h3>Test your button</h3>
    <p>Press a mapped button on your remote, then come back here.</p>
    <button class="primary" disabled={busy} onclick={checkTest}
      >I pressed it</button
    >
    {#if testResult}<p role="status">{testResult}</p>{/if}
    <button class="secondary" onclick={() => (view = "list")}>Done</button>
  </div>
{:else}
  {#if tv.mappings.length}<p class="subtle" role="status">{status}</p>
    {#if tv.mappings.length && !tv.running && !tv.paused}<button
        class="secondary"
        onclick={() => change({ kind: "apply", mappings: tv!.mappings })}
        >Start it again</button
      >{/if}
    <div class="row-list app-list">
      {#each tv.mappings as mapping (describeKey(mapping.key))}<div
          class="file-row"
        >
          <div class="list-row entry-static">
            <span class="truncate"
              ><span class="entry-name">{mapping.label}</span><small
                >{describeKey(mapping.key)}</small
              ></span
            >
          </div>
          <button
            class="icon-button file-more"
            aria-label={`Remove ${mapping.label}`}
            disabled={busy}
            onclick={() => remove(mapping)}><Icon name="close" /></button
          >
        </div>{/each}
    </div>
  {:else}<section class="center-tv">
      <h2>No buttons remapped yet</h2>
      <p class="subtle">
        Make any button on your remote open the app you choose.
      </p>
    </section>{/if}
  {#if toolkit && toolkit.stage !== "done"}<p class="subtle">
      Mappings only stay running after setup is finished.
      <button class="text-action" onclick={onsetup}>Finish TV setup</button>
    </p>{/if}
  <button
    class="primary"
    onclick={() => {
      listened = false;
      presses = [];
      view = "listen";
    }}>Remap a button</button
  >
  {#if tv.mappings.length}<div class="page-footer">
      <button class="secondary" disabled={busy} onclick={startTest}
        >Test a button</button
      ><button
        class="secondary"
        disabled={busy}
        onclick={() => change({ kind: "pause", paused: !tv!.paused })}
        >{tv.paused ? "Resume" : "Pause"}</button
      ><button
        class="secondary"
        disabled={busy}
        onclick={() => change({ kind: "removeAll" })}>Remove all</button
      >
    </div>{/if}
{/if}
