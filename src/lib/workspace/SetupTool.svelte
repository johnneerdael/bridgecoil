<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type InstallProgress,
    type Manager,
    type MappingState,
    type Platform,
    type Review,
    type SetupStep,
    type SetupWatch,
    type Stage,
    type ToolId,
    type ToolkitSnapshot,
  } from "./client";
  import Icon from "./Icon.svelte";
  let {
    device,
    api,
    reload,
    onreview,
    onbuttons,
    watching = null,
    onwatch = () => {},
    onwatched = () => {},
    pollMs = 3000,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    onbuttons: () => void;
    watching?: SetupWatch | null;
    onwatch?: (next: SetupWatch | null) => void;
    onwatched?: () => void;
    pollMs?: number;
  } = $props();
  const steps: { id: Stage; title: string }[] = [
    { id: "manager", title: "Choose a helper app" },
    { id: "server", title: "Start it" },
    { id: "terminal", title: "Set up Termux" },
    { id: "shell", title: "Connect Termux" },
    { id: "access", title: "Allow access" },
    { id: "startup", title: "Survive restarts" },
  ];
  const managers: { id: Exclude<Manager, "existing">; name: string; note: string }[] = [
    {
      id: "shizukuNext",
      name: "Shizuku-Next",
      note: "Full-featured, with a TV-friendly screen. Replaces other Shizuku apps.",
    },
    {
      id: "porter",
      name: "Porter",
      note: "Small and focused. Installs alongside other Shizuku apps.",
    },
  ];
  const stageLabels: Record<string, string> = {
    downloading: "Downloading and checking the latest release…",
    uploading: "Copying to the TV…",
    installing: "Installing on the TV…",
    verifying: "Confirming it installed…",
  };
  let snapshot = $state<ToolkitSnapshot | null>(null);
  let refresh = $state(0);
  let busy = $state(false);
  let error = $state("");
  let installing = $state<InstallProgress | null>(null);
  let survived = $state(false);
  let bootChecked = $state(false);
  const accessTries = 40;
  const bootTries = 100;
  const current = $derived(steps.findIndex((s) => s.id === snapshot?.stage));
  const finished = $derived(snapshot?.stage === "done");
  const managerName = $derived(
    snapshot?.manager === "existing"
      ? (snapshot.helper?.label ?? "your helper app")
      : (managers.find((m) => m.id === snapshot?.manager)?.name ?? ""),
  );
  const allowLabel = $derived(
    snapshot?.manager === "porter" ? "Allow all the time" : "Always allow",
  );
  $effect(() => {
    const target = targetOf(device);
    reload;
    refresh;
    let cancelled = false;
    void api
      .query<ToolkitSnapshot>(target, { kind: "toolkit" })
      .then(async (r) => {
        if (cancelled) return;
        snapshot = r;
        if (r.stage !== "done") return;
        // Only adds a reassurance, so failing to read it changes nothing.
        const state = await api
          .query<MappingState>(target, { kind: "mappings" })
          .catch(() => null);
        if (!cancelled) survived = state?.survivedRestart ?? false;
      })
      .catch((e) => {
        if (!cancelled) error = errorMessage(e);
      });
    return () => {
      cancelled = true;
    };
  });
  async function install(tool: ToolId) {
    busy = true;
    error = "";
    installing = { stage: "downloading", sent: 0, total: 0 };
    try {
      await api.installTool(device.id, tool, (p) => (installing = p));
      refresh++;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
      installing = null;
    }
  }
  // The TV is where these get answered, so setup watches it instead of asking for a button.
  $effect(() => {
    if (watching !== "access" || snapshot?.stage !== "access") return;
    const target = targetOf(device);
    let stopped = false;
    void (async () => {
      for (let tries = 0; tries < accessTries && !stopped; tries++) {
        const r = await api
          .query<{ route: string }>(target, { kind: "verifyShell" })
          .catch(() => null);
        if (stopped) return;
        if (r?.route === "shell" || r?.route === "root") {
          onwatched();
          refresh++;
          return;
        }
        await new Promise((done) => setTimeout(done, pollMs));
      }
      if (stopped) return;
      onwatched();
      error = `Termux still can't use ${managerName}. Choose ${allowLabel} on the TV, then check again.`;
    })();
    return () => {
      stopped = true;
    };
  });
  $effect(() => {
    if (watching !== "boot" || snapshot?.stage === undefined) return;
    if (snapshot.bootStart) {
      onwatched();
      return;
    }
    let tries = 0;
    const timer = setInterval(() => {
      if (++tries > bootTries) {
        clearInterval(timer);
        bootChecked = true;
        onwatched();
      } else refresh++;
    }, pollMs);
    return () => clearInterval(timer);
  });
  async function run(step: SetupStep, watch: SetupWatch | null = null) {
    onwatch(watch);
    busy = true;
    error = "";
    try {
      onreview(
        await api.prepare(targetOf(device), { kind: "setup", step }),
      );
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function checkAccess() {
    busy = true;
    error = "";
    try {
      const { route } = await api.query<{ route: string }>(targetOf(device), {
        kind: "verifyShell",
      });
      if (route === "none")
        error = `Termux hasn't been allowed yet. Choose ${allowLabel} on the TV, then check again.`;
      else if (route === "shell" || route === "root") refresh++;
      else error = "The TV didn't answer in time. Check access again.";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const press = (key: string) =>
    api.remote(targetOf(device), { kind: "key", key, longPress: false });
</script>

{#snippet stopUsingHelper()}
  {#if snapshot?.manager === "existing"}<button
      class="secondary"
      disabled={busy}
      onclick={() => run({ kind: "stopUsingHelper" })}
      >Stop using {managerName}</button
    >{/if}
{/snippet}

{#snippet startNote()}
  {#if !snapshot?.bootStart && !snapshot?.networkAdb && (snapshot?.noWirelessDebugging || snapshot?.wired)}<p
      class="risk-note"
      role="note"
    >
      {#if snapshot?.noWirelessDebugging}This TV has no network ADB turned on, so
        {managerName} may not start by itself.{:else}{managerName} can't start
        by itself on a network cable. Use Wi-Fi if you can.{/if}
      Otherwise, start it here after each restart.
    </p>{/if}
{/snippet}

{#if !snapshot && !error}<p class="subtle" role="status">Checking the TV…</p>
{:else if snapshot}
  <ol class="setup-steps">
    {#each steps as step, index}<li
        class="setup-step"
        class:done={finished || index < current}
        class:current={index === current}
      >
        <Icon name={finished || index < current ? "check" : "minus"} size={16} />
        <span>{step.title}</span>
      </li>{/each}
  </ol>
  {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  {#if installing}<p class="subtle" role="status">
      {stageLabels[installing.stage] ?? "Working…"}
      {#if installing.stage === "installing"}Look at the TV in case it's asking
        you something.{/if}
    </p>{/if}
  <div class="setup-card">
    {#if snapshot.stage === "manager"}
      <p>
        These helper apps give your TV the access it needs. Pick one and
        Bridgecoil gets the latest version for you.
      </p>
      {#if snapshot.helper}<div class="setup-choice">
          <strong>{snapshot.helper.label} is already on this TV</strong>
          <p class="subtle">
            You can use it instead. Bridgecoil hasn't verified this build, so
            only do this if you trust where it came from.
          </p>
          <button
            class="secondary"
            disabled={busy}
            onclick={() => run({ kind: "useHelper" })}
            >Use {snapshot.helper.label}</button
          >
        </div>{/if}
      {#if snapshot.shizuku === "other"}<p class="subtle">
          A different app is using the Shizuku name on this TV, so Shizuku-Next
          can't be installed. Porter can sit alongside it.
        </p>{/if}
      {#if snapshot.shizuku === "unknown"}<p class="subtle">
          Bridgecoil couldn't check which helper app is installed. Tap refresh
          to try again.
        </p>{/if}
      {#each managers as manager}<div class="setup-choice">
          <strong>{manager.name}</strong>
          <p class="subtle">{manager.note}</p>
          <button
            class="secondary"
            disabled={busy ||
              (manager.id === "shizukuNext" &&
                ["other", "unknown"].includes(snapshot.shizuku))}
            aria-label={`Install ${manager.name}`}
            onclick={() => install(manager.id)}>Install</button
          >
        </div>{/each}
    {:else if snapshot.stage === "server" && snapshot.manager}
      <p>
        {managerName} needs its background service running. Bridgecoil can start
        it over this connection.
      </p>
      <button
        class="primary"
        disabled={busy}
        onclick={() =>
          run({ kind: "startServer", manager: snapshot!.manager! })}
        >Start {managerName}</button
      >
    {:else if snapshot.stage === "terminal"}
      <p>
        Termux keeps your button mappings running on the TV. You won't need to
        type anything in it.
      </p>
      {#if !snapshot.termux.installed || !snapshot.termuxBoot.installed}<p
          class="risk-note"
          role="note"
        >
          Google Play Protect may stop this on the TV with "Unsafe app blocked"
          or "Blocked by Play Protect", because Termux is built for an older
          version of Android. Bridgecoil has checked it really is Termux. To
          continue, use your remote on the TV to open
          <strong>More details</strong> or <strong>Details</strong>, then choose
          <strong>Install anyway</strong>.
        </p>{/if}
      {#if snapshot.termux.installed && !snapshot.termux.official}<p
          class="error-text"
        >
          A different Termux build is on this TV, and Bridgecoil can't set that
          one up. Remove it in Apps to continue, which also deletes its files.
        </p>
      {:else if !snapshot.termux.installed}<button
          class="primary"
          disabled={busy}
          onclick={() => install("termux")}>Install Termux</button
        >
      {:else if snapshot.termuxBoot.installed && !snapshot.termuxBoot.official}<p
          class="error-text"
        >
          A different Termux:Boot build is on this TV. Remove it in Apps to
          continue.
        </p>
      {:else if !snapshot.termuxBoot.installed}<button
          class="primary"
          disabled={busy}
          onclick={() => install("termuxBoot")}>Install Termux:Boot</button
        >
      {:else if !snapshot.canConfigureTerminal}<p class="error-text">
          This TV blocks the access Bridgecoil needs to set up Termux, so button
          mappings can't be kept running here. You can still use them until the
          TV restarts.
        </p>
      {:else}<button
          class="primary"
          disabled={busy}
          onclick={() => run({ kind: "openTerminal" })}>Prepare Termux</button
        >
        <p class="subtle">
          Termux opens on the TV once. If this step doesn't finish, give it a
          minute and try again.
        </p>{/if}
    {:else if snapshot.stage === "shell" && snapshot.manager}
      <p>Next, Termux needs the shell tool from {managerName}.</p>
      <button
        class="primary"
        disabled={busy}
        onclick={() =>
          run({ kind: "deployShell", manager: snapshot!.manager! })}
        >Connect Termux</button
      >
    {:else if snapshot.stage === "access"}
      <p>
        {managerName} will ask on the TV whether Termux can use it. Choose
        <strong>{allowLabel}</strong>.
      </p>
      {#if watching === "access"}<p role="status">
          Waiting for the TV. This moves on by itself.
        </p>
      {:else}<button
          class="primary"
          disabled={busy}
          onclick={() =>
            run(
              { kind: "requestAccess", manager: snapshot!.manager! },
              "access",
            )}>Ask the TV</button
        >{/if}
      <p class="subtle">Can't reach the TV remote? Use these.</p>
      <div class="setup-remote">
        {#each [["left", "Left"], ["ok", "OK"], ["right", "Right"]] as [key, label]}<button
            class="secondary"
            onclick={() => press(key)}>{label}</button
          >{/each}
      </div>
      {#if error}<button
          class="secondary"
          disabled={busy}
          onclick={checkAccess}>Check access</button
        >{/if}
    {:else if snapshot.stage === "startup" && snapshot.manager}
      <h3>{snapshot.monitor ? "Not ready for restarts yet" : "Survive restarts"}</h3>
      {@render startNote()}
      <ul class="setup-checks">
        <li class:done={snapshot.manager !== "shizukuNext" || snapshot.watchdog}>
          <Icon
            name={snapshot.manager !== "shizukuNext" || snapshot.watchdog
              ? "check"
              : "minus"}
            size={16}
          />
          <span
            >Keep running<small>
              {#if snapshot.manager === "shizukuNext"}Watchdog
                {snapshot.watchdog ? "on" : "off"}{:else}Battery limits{/if}</small
            ></span
          >
          <button
            class="secondary"
            disabled={busy}
            aria-label={`Let Termux and ${managerName} keep running`}
            onclick={() =>
              run({ kind: "allowBackground", manager: snapshot!.manager! })}
            >Allow</button
          >
        </li>
        <li class:done={snapshot.bootStart}>
          <Icon name={snapshot.bootStart ? "check" : "minus"} size={16} />
          <span
            >Start on boot<small>
              {#if snapshot.bootStart}On
              {:else if watching === "boot"}Waiting for you to turn it on: Settings, Start on boot
              {:else if bootChecked}Still off. On the TV: Settings, Start on boot
              {:else}On the TV: Settings, Start on boot{/if}</small
            ></span
          >
          {#if !snapshot.bootStart}<button
              class="secondary"
              disabled={busy}
              aria-label={`Open ${managerName} on the TV`}
              onclick={() =>
                run(
                  { kind: "allowBootStart", manager: snapshot!.manager! },
                  "boot",
                )}>Open</button
            >{/if}
        </li>
        <li class:done={snapshot.monitor}>
          <Icon name={snapshot.monitor ? "check" : "minus"} size={16} />
          <span>Restore mappings<small>Runs when the TV starts</small></span>
          {#if !snapshot.monitor}<button
              class="secondary"
              disabled={busy}
              aria-label="Keep mappings running"
              onclick={() => run({ kind: "installMonitor" })}>Add</button
            >{/if}
        </li>
      </ul>
      {#if !snapshot.bootStart && watching !== "boot" && bootChecked}<button
          class="primary"
          disabled={busy}
          onclick={() => refresh++}>Check start on boot</button
        >{/if}
      {#if snapshot.monitor}<button class="secondary" onclick={onbuttons}
          >Remap buttons</button
        >{/if}
      {@render stopUsingHelper()}
    {:else if snapshot.stage === "done"}
      <h3>Your TV is ready</h3>
      <p>Everything is in place to remap buttons.</p>
      {#if survived}<p class="subtle">
          Proven on this TV: your button mappings came back by themselves after
          it restarted.
        </p>
      {:else}<p class="subtle">
          Recovery after a restart hasn't been proven on this TV yet. After you
          map a button, restart the TV and open Buttons to see if it came back.
        </p>{/if}
      <button class="primary" onclick={onbuttons}>Remap buttons</button>
      <button
        class="secondary"
        disabled={busy}
        onclick={() => run({ kind: "removeMonitor" })}
        >Remove the startup script</button
      >
      {@render stopUsingHelper()}
    {/if}
  </div>
{:else}<p class="error-text" role="alert">{error}</p>{/if}
