<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type InstallProgress,
    type LatestLauncher,
    type Platform,
    type Review,
    type ToolId,
  } from "./client";
  let {
    device,
    api,
    reload,
    onreview,
    oninstall,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    oninstall: () => void;
  } = $props();
  type Listed = {
    tool: ToolId;
    name: string;
    package: string;
    installed: boolean;
    version: string | null;
    official: boolean | null;
  };
  type Homes = {
    homes: string[];
    current: string | null;
    stock: string[];
    chosen?: string | null;
    roles?: boolean;
    catalog?: Listed[];
    restorable?: { package: string; component: string | null }[];
    overriding?: string[];
  };
  let homes = $state<Homes | null>(null);
  let busy = $state(false);
  let error = $state("");
  let tried = $state<string | null>(null);
  let result = $state<"proven" | "wrong" | null>(null);
  let latest = $state<LatestLauncher[]>([]);
  let installing = $state<InstallProgress | null>(null);
  let refresh = $state(0);
  const packageOf = (component: string) => component.split("/")[0];
  const catalog = $derived(homes?.catalog ?? []);
  const restorable = $derived(homes?.restorable ?? []);
  const overriding = $derived(homes?.overriding ?? []);
  const nameOf = (component: string) =>
    catalog.find((c) => c.package === packageOf(component))?.name ??
    packageOf(component);
  const componentOf = (listed: Listed) =>
    homes?.homes.find((h) => packageOf(h) === listed.package);
  const latestOf = (listed: Listed) =>
    latest.find((l) => l.tool === listed.tool)?.version;
  // Tags and versions are dotted numbers. Anything else isn't offered as an update.
  function isNewer(next: string, now: string) {
    const parts = (v: string) => v.split(".").map(Number);
    const [a, b] = [parts(next), parts(now)];
    if ([...a, ...b].some(Number.isNaN)) return false;
    for (let i = 0; i < Math.max(a.length, b.length); i++) {
      const difference = (a[i] ?? 0) - (b[i] ?? 0);
      if (difference) return difference > 0;
    }
    return false;
  }
  // A build signed by someone else can't be updated from here, so it's left alone.
  const updateFor = (listed: Listed) => {
    const next = latestOf(listed);
    return listed.official && listed.version && next && isNewer(next, listed.version)
      ? next
      : null;
  };
  function statusOf(listed: Listed) {
    if (!listed.installed) return latestOf(listed) ?? "Not installed";
    if (listed.official === false) return "Your own build";
    const update = updateFor(listed);
    if (update) return `${listed.version} → ${update}`;
    return listed.version ?? "Installed";
  }
  const stageLabels: Record<string, string> = {
    downloading: "Downloading and checking the latest release…",
    uploading: "Copying to the TV…",
    installing: "Installing on the TV…",
    verifying: "Confirming it installed…",
  };
  const onStock = $derived(!!homes && homes.stock.includes(homes.current ?? ""));
  // A TV with no role service can't say what was chosen, so it's kept on the phone.
  const pickKey = $derived(`bridgecoil.home.${device.deviceId}`);
  function readPick() {
    try {
      return localStorage.getItem(pickKey);
    } catch {
      return null;
    }
  }
  function keepPick(component: string) {
    try {
      if (homes?.stock.includes(component)) localStorage.removeItem(pickKey);
      else localStorage.setItem(pickKey, packageOf(component));
    } catch {
      // Without storage, this TV falls back to the list of launchers.
    }
  }
  // The Home button is only asked about once for each launcher, since a page reload loses `result`.
  const provenKey = $derived(`bridgecoil.home.proven.${device.deviceId}`);
  let proven = $state<string | null>(null);
  $effect(() => {
    try {
      proven = localStorage.getItem(provenKey);
    } catch {
      proven = null;
    }
  });
  function prove() {
    const inUse = packageOf(homes?.current ?? "");
    result = "proven";
    proven = inUse;
    try {
      localStorage.setItem(provenKey, inUse);
    } catch {
      // Without storage, the question comes back next time.
    }
  }
  const confirmed = $derived(
    !onStock && !!homes?.current && proven === packageOf(homes.current),
  );
  const chosen = $derived(
    homes?.roles === false
      ? homes.homes.map(packageOf).find((p) => p === readPick())
      : homes?.chosen,
  );
  // Android ignores the choice when the TV's own launcher has a higher priority.
  const outranked = $derived(
    onStock && !!chosen && chosen !== packageOf(homes?.current ?? ""),
  );
  const others = $derived(homes?.homes.filter((h) => h !== homes!.current) ?? []);
  // Launchers Bridgecoil doesn't list, such as one installed from a file.
  const unlisted = $derived(
    others.filter((h) => !catalog.some((c) => c.package === packageOf(h))),
  );
  const stockLeft = $derived(homes?.stock.filter((h) => h !== homes!.current) ?? []);
  $effect(() => {
    let cancelled = false;
    // Without this, the list still works and just offers no updates.
    void api
      .latestLaunchers()
      .then((r) => {
        if (!cancelled) latest = r;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  });
  $effect(() => {
    const target = targetOf(device);
    reload;
    refresh;
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
  const install = (listed: Listed) =>
    attempt(async () => {
      installing = { stage: "downloading", sent: 0, total: 0 };
      try {
        await api.installTool(device.id, listed.tool, (p) => (installing = p));
        refresh++;
      } finally {
        installing = null;
      }
    });
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
      keepPick(component);
      onreview(
        await api.prepare(targetOf(device), {
          kind: "defaultHome",
          component,
        }),
      );
    });
  // The old launcher goes off in the same step, or it can take Home straight back.
  const switchTo = (component: string) =>
    attempt(async () => {
      keepPick(component);
      // Everything that could keep Home goes off, so this can't report success with one left on.
      const inUse = packageOf(homes?.current ?? "");
      const launcher = homes?.homes.some((h) => packageOf(h) === inUse);
      const replacing = [...new Set([...(launcher ? [inUse] : []), ...overriding])].filter(
        (p) => p !== packageOf(component),
      );
      onreview(
        await api.prepare(targetOf(device), {
          kind: "switchHome",
          component,
          replacing,
        }),
      );
      // The launcher that was in charge is off, so there's nothing left to ask about Home.
      try {
        localStorage.setItem(provenKey, packageOf(component));
      } catch {
        // Without storage, the question comes back next time.
      }
    });
  // Without a component, the launcher's own priority takes Home back once it's on.
  const restore = (listed: { package: string; component: string | null }) =>
    attempt(async () => {
      try {
        localStorage.removeItem(pickKey);
      } catch {
        // There's nothing to forget without storage.
      }
      onreview(
        await api.prepare(
          targetOf(device),
          listed.component
            ? { kind: "restoreHome", component: listed.component }
            : { kind: "package", package: listed.package, action: "enable" },
        ),
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

{#snippet restoreButtons()}
  {#each restorable as listed (listed.package)}<button
      class="secondary"
      disabled={busy}
      onclick={() => restore(listed)}
      >Restore your TV's launcher{restorable.length > 1
        ? ` (${listed.package})`
        : ""}</button
    >{/each}
{/snippet}
{#snippet wayBack()}
  {#each others as component (component)}<button
      class="secondary"
      disabled={busy}
      onclick={() => switchTo(component)}
      >Use {nameOf(component)} instead</button
    >{:else}{#if restorable.length}{@render restoreButtons()}{:else}<p class="subtle">
        To go back, undo turning off the launcher your TV came with in History,
        or turn it back on in Apps, under Disabled apps.
      </p>{/if}{/each}
{/snippet}

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if !homes}{#if !error}<p class="subtle" role="status">Checking the TV…</p>{/if}
{:else if tried}
  <div class="setup-card">
    <h3>Did it open?</h3>
    <p>
      {nameOf(tried)} should be on your TV now. Try moving around with your
      remote. Nothing has been changed yet.
    </p>
    <button
      class="primary"
      disabled={busy}
      onclick={() => {
        const component = tried!;
        tried = null;
        return switchTo(component);
      }}>Yes, it works</button
    >
    <button class="secondary" onclick={() => (tried = null)}>No</button>
  </div>
{:else if overriding.length}
  <div class="setup-card">
    <h3>{nameOf(overriding[0])} is still taking over Home</h3>
    <p>
      Your TV's default is {homes.current ? nameOf(homes.current) : "another launcher"},
      but {nameOf(overriding[0])} takes the Home button back. Turning it off ends
      that. You can undo this from History or Apps.
    </p>
    <button
      class="primary"
      disabled={busy}
      onclick={() =>
        homes?.current ? switchTo(homes.current) : turnOff(overriding[0])}
      >Turn off {nameOf(overriding[0])}</button
    >
  </div>
{:else if outranked && homes.current}
  {@const inUse = homes.current}
  <div class="setup-card">
    <h3>Your TV's own launcher comes first</h3>
    <p>
      You chose {chosen}, but this TV gives {nameOf(inUse)} priority,
      so the Home button keeps opening it. Turning it off hands Home over. Some
      TV features can depend on it. You can undo this from History or Apps.
    </p>
    <button
      class="primary"
      disabled={busy}
      onclick={() => {
        const next = homes?.homes.find((h) => packageOf(h) === chosen);
        return next ? switchTo(next) : turnOff(inUse);
      }}>Turn off {nameOf(inUse)}</button
    >
    <button
      class="secondary"
      disabled={busy}
      onclick={() => makeDefault(inUse)}>Keep {nameOf(inUse)}</button
    >
  </div>
{:else if !onStock && result === null && homes.current && !confirmed}
  <div class="setup-card">
    <h3>Press Home on your remote</h3>
    <p>
      Asking for a new launcher doesn't prove the Home button uses it. Press
      the physical Home button and see what opens.
    </p>
    <button class="primary" onclick={prove}>Home opened it</button>
    <button class="secondary" onclick={() => (result = "wrong")}
      >Home opened something else</button
    >
  </div>
{:else if result === "wrong"}
  <div class="setup-card">
    <h3>This TV kept its own Home</h3>
    <p>Some TVs don't let the Home button change.</p>
    {@render wayBack()}
  </div>
{:else}
  {#if result === "proven" || (confirmed && stockLeft.length)}<div class="setup-card">
      <h3>Your launcher is in charge</h3>
      {#if stockLeft.length}<p>
          You can switch off the launcher your TV came with. Some TV features can
          depend on it. You can undo this from History or Apps.
        </p>
        {#each stockLeft as component (component)}<button
            class="secondary"
            disabled={busy}
            onclick={() => turnOff(component)}
            >Turn off {nameOf(component)}</button
          >{/each}{/if}
      {@render wayBack()}
    </div>{/if}
  <p class="subtle">In use: {homes.current ? nameOf(homes.current) : "the TV didn't say"}</p>
  {#if installing}<p class="subtle" role="status">
      {stageLabels[installing.stage] ?? "Working…"}
    </p>{/if}
  <div class="row-list app-list">
    {#each catalog as listed (listed.tool)}
      {@const component = componentOf(listed)}
      {@const update = updateFor(listed)}
      <div class="list-row">
        <span class="truncate"
          >{listed.name}<small class="subtle">{statusOf(listed)}</small></span
        >
        {#if !listed.installed}<button
            class="secondary"
            disabled={busy}
            aria-label={`Install ${listed.name}`}
            onclick={() => install(listed)}>Install</button
          >{/if}
        {#if update}<button
            class="secondary"
            disabled={busy}
            aria-label={`Update ${listed.name}`}
            onclick={() => install(listed)}>Update</button
          >{/if}
        {#if component && component !== homes.current}<button
            class="secondary"
            disabled={busy}
            aria-label={`Try ${listed.name}`}
            onclick={() => tryOut(component)}>Try it</button
          >{/if}
      </div>
    {/each}
    {#each unlisted as component (component)}<div class="list-row">
        <span class="truncate">{packageOf(component)}</span><button
          class="secondary"
          disabled={busy}
          aria-label={`Try ${packageOf(component)}`}
          onclick={() => tryOut(component)}>Try it</button
        >
      </div>{/each}
  </div>
  <button class="secondary" disabled={busy} onclick={oninstall}
    >Install from a file</button
  >
  {@render restoreButtons()}
{/if}
