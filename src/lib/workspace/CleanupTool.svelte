<script lang="ts" module>
  // Applying a review remounts this screen, and a choice to keep an app has to survive that.
  const keptByDevice = new Map<string, string[]>();
</script>

<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type AppMetadata,
    type BulkReview,
    type PackageAction,
    type Page,
    type Platform,
  } from "./client";
  let {
    device,
    api,
    reload,
    onbulk,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onbulk: (r: BulkReview) => void;
  } = $props();
  // The most one bulk review accepts.
  const reviewLimit = 32;
  const pageSize = 32;
  let hasRatings = $state<boolean | null>(null);
  let offer = $state<AppMetadata[]>([]);
  let turnedOff = $state<AppMetadata[]>([]);
  // svelte-ignore state_referenced_locally
  let skipped = $state<string[]>(keptByDevice.get(device.deviceId) ?? []);
  let loaded = $state(false);
  let busy = $state(false);
  let error = $state("");
  let refresh = $state(0);
  const chosen = $derived(
    offer
      .filter((app) => !skipped.includes(app.package))
      .map((app) => app.package)
      .slice(0, reviewLimit),
  );
  const leftOver = $derived(
    offer.filter((app) => !skipped.includes(app.package)).length -
      chosen.length,
  );
  const plural = (n: number) => (n === 1 ? "app" : "apps");
  async function readAll(
    filter: string,
    keep: (app: AppMetadata) => boolean,
  ): Promise<AppMetadata[]> {
    const found: AppMetadata[] = [];
    for (let page = 0; ; page++) {
      const result = await api.query<Page<AppMetadata>>(targetOf(device), {
        kind: "packages",
        filter,
        search: "",
        page,
        pageSize,
      });
      const kept = result.items.filter(keep);
      found.push(...kept);
      // Rated groups come back in order, so the first other group ends the search.
      if (filter === "system" && kept.length < result.items.length) break;
      if ((page + 1) * pageSize >= result.total || !result.items.length) break;
    }
    return found;
  }
  $effect(() => {
    reload;
    refresh;
    let cancelled = false;
    void (async () => {
      try {
        const status = await api.ratingsStatus();
        if (cancelled) return;
        hasRatings = status.stored;
        if (!status.stored) return;
        const off = await readAll("disabled", () => true);
        const candidates = await readAll(
          "system",
          (app) => app.group === "recommended",
        );
        if (cancelled) return;
        const disabled = new Set(off.map((app) => app.package));
        offer = candidates.filter((app) => !disabled.has(app.package));
        // Only what this tool would turn off. Some TVs ship with other apps disabled on purpose.
        turnedOff = off.filter((app) => app.group === "recommended");
        loaded = true;
      } catch (e) {
        if (!cancelled) error = errorMessage(e);
      }
    })();
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
  const review = (packages: string[], action: PackageAction) =>
    attempt(async () =>
      onbulk(await api.prepareBulk(targetOf(device), packages, action)),
    );
  const download = () =>
    attempt(async () => {
      await api.ratingsUpdate();
      refresh++;
    });
  function toggle(id: string) {
    skipped = skipped.includes(id)
      ? skipped.filter((p) => p !== id)
      : [...skipped, id];
    keptByDevice.set(device.deviceId, skipped);
  }
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if hasRatings === false}
  <div class="setup-card">
    <h3>Find what's safe to turn off</h3>
    <p>
      Bridgecoil uses a community list of which preinstalled apps can go. It's
      a small download from GitHub.
    </p>
    <button class="primary" disabled={busy} onclick={download}
      >Download ratings</button
    >
  </div>
{:else if !loaded}{#if !error}<p class="subtle" role="status">
      Checking the TV…
    </p>{/if}
{:else}
  {#if offer.length}
    <p class="subtle">
      These came with your TV, and the community rates them safe to turn off.
      Nothing is deleted, and you can turn them back on here.
    </p>
    <div class="row-list app-list">
      {#each offer as app (app.package)}<label
          class="checkbox list-row bulk-choice"
          ><input
            type="checkbox"
            checked={!skipped.includes(app.package)}
            disabled={busy}
            aria-label={`Turn off ${app.label || app.package}`}
            onchange={() => toggle(app.package)}
          /><span class="truncate"
            ><span class="entry-name">{app.label || app.package}</span>{#if app
              .rating?.description}<small>{app.rating.description}</small
              >{/if}</span
          ></label
        >{/each}
    </div>
    {#if leftOver > 0}<p class="subtle">
        {leftOver} more can follow once these are done.
      </p>{/if}
    <button
      class="primary"
      disabled={busy || !chosen.length}
      onclick={() => review(chosen, "disable")}
      >Review {chosen.length} {plural(chosen.length)}</button
    >
  {:else}<section class="center-tv">
      <h2>Nothing left to clean up</h2>
      <p class="subtle">
        No apps on this TV are rated safe to turn off. The rest are listed in
        Apps with what's known about each.
      </p>
    </section>{/if}
  {#if turnedOff.length}<p class="subtle">
      {turnedOff.length}
      {plural(turnedOff.length)} turned off on this TV.
    </p>
    <button
      class="secondary"
      disabled={busy}
      onclick={() =>
        review(
          turnedOff.slice(0, reviewLimit).map((app) => app.package),
          "enable",
        )}
      >Turn {Math.min(turnedOff.length, reviewLimit)} back on</button
    >{/if}
{/if}
