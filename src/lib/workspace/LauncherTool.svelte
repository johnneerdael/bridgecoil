<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import { targetOf, type Platform, type Review } from "./client";
  let {
    device,
    api,
    reload,
    onreview,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
  } = $props();
  type Homes = { homes: string[]; current: string | null; stock: string[] };
  let homes = $state<Homes | null>(null);
  let busy = $state(false);
  let error = $state("");
  let tried = $state<string | null>(null);
  let result = $state<"proven" | "wrong" | null>(null);
  const packageOf = (component: string) => component.split("/")[0];
  const onStock = $derived(!!homes && homes.stock.includes(homes.current ?? ""));
  const others = $derived(homes?.homes.filter((h) => h !== homes!.current) ?? []);
  const stockLeft = $derived(homes?.stock.filter((h) => h !== homes!.current) ?? []);
  $effect(() => {
    const target = targetOf(device);
    reload;
    let cancelled = false;
    void api
      .query<Homes>(target, { kind: "launchers" })
      .then((r) => {
        if (!cancelled) homes = r;
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
  const tryOut = (component: string) =>
    attempt(async () => {
      const r = await api.remote(targetOf(device), {
        kind: "launch",
        component,
      });
      if (r.status === "sent") tried = component;
      else error = r.error || "The TV didn't open that launcher.";
    });
  const makeDefault = (component: string) =>
    attempt(async () => {
      onreview(
        await api.prepare(targetOf(device), {
          kind: "defaultHome",
          component,
        }),
      );
    });
  const turnOff = (component: string) =>
    attempt(async () => {
      onreview(
        await api.prepare(targetOf(device), {
          kind: "package",
          package: packageOf(component),
          action: "disable",
        }),
      );
    });
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if !homes}{#if !error}<p class="subtle" role="status">Checking the TV…</p>{/if}
{:else if tried}
  <div class="setup-card">
    <h3>Did it open?</h3>
    <p>
      {packageOf(tried)} should be on your TV now. Try moving around with your
      remote. Nothing has been changed yet.
    </p>
    <button
      class="primary"
      disabled={busy}
      onclick={() => {
        const component = tried!;
        tried = null;
        return makeDefault(component);
      }}>Yes, it works</button
    >
    <button class="secondary" onclick={() => (tried = null)}>No</button>
  </div>
{:else if !onStock && result === null && homes.current}
  <div class="setup-card">
    <h3>Press Home on your remote</h3>
    <p>
      Asking for a new launcher doesn't prove the Home button uses it. Press
      the physical Home button and see what opens.
    </p>
    <button class="primary" onclick={() => (result = "proven")}
      >Home opened it</button
    >
    <button class="secondary" onclick={() => (result = "wrong")}
      >Home opened something else</button
    >
  </div>
{:else if result === "wrong"}
  <div class="setup-card">
    <h3>This TV kept its own Home</h3>
    <p>Some TVs don't let the Home button change. You can go back.</p>
    {#each stockLeft as component (component)}<button
        class="primary"
        disabled={busy}
        onclick={() => makeDefault(component)}
        >Go back to {packageOf(component)}</button
      >{/each}
  </div>
{:else if result === "proven"}
  <div class="setup-card">
    <h3>Your launcher is in charge</h3>
    {#if stockLeft.length}<p>
        You can switch off the launcher your TV came with. Some TV features can
        depend on it. You can undo this from History or Apps.
      </p>
      {#each stockLeft as component (component)}<button
          class="secondary"
          disabled={busy}
          onclick={() => turnOff(component)}
          >Turn off {packageOf(component)}</button
        >{/each}{/if}
  </div>
{:else}
  <p class="subtle">In use: {homes.current ?? "the TV didn't say"}</p>
  {#if !others.length}<p>
      This is the only launcher on the TV. Install another from Apps, and it
      will appear here.
    </p>{/if}
  <div class="row-list app-list">
    {#each others as component (component)}<div class="list-row">
        <span class="truncate">{packageOf(component)}</span><button
          class="secondary"
          disabled={busy}
          aria-label={`Try ${packageOf(component)}`}
          onclick={() => tryOut(component)}>Try it</button
        >
      </div>{/each}
  </div>
{/if}
