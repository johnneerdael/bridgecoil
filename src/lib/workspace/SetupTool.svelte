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
    pollMs = 3000,
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    onbuttons: () => void;
    pollMs?: number;
  } = $props();
  const steps: { id: Stage; title: string }[] = [
    { id: "terminal", title: "Set up Termux" },
    { id: "pair", title: "Connect to the TV" },
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
  let code = $state("");
  let agentTried = false;
  const approvalTries = 40;
  const current = $derived(steps.findIndex((s) => s.id === snapshot?.stage));
  const finished = $derived(snapshot?.stage === "done");
  const managerName = $derived(
    snapshot?.manager === "existing"
      ? (snapshot.helper?.label ?? "your helper app")
      : (managers.find((m) => m.id === snapshot?.manager)?.name ?? ""),
  );
  // A TV with no wireless debugging can't show a code, so it approves the connection instead.
  const approves = $derived(!!snapshot?.noWirelessDebugging);
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
  $effect(() => {
    if (!snapshot || snapshot.stage === "terminal" || snapshot.agent || agentTried) return;
    agentTried = true;
    void api
      .installAgent(device.id)
      .then(() => refresh++)
      .catch((e) => (error = errorMessage(e)));
  });
  // The TV answers this, so setup watches it instead of asking for a button.
  $effect(() => {
    if (snapshot?.stage !== "pair" || !snapshot.agent || !approves) return;
    const target = targetOf(device);
    let stopped = false;
    void (async () => {
      for (let tries = 0; tries < approvalTries && !stopped; tries++) {
        const r = await api
          .query<{ route: string }>(target, { kind: "verifyAgent" })
          .catch(() => null);
        if (stopped) return;
        if (r?.route === "shell") {
          refresh++;
          return;
        }
        await new Promise((done) => setTimeout(done, pollMs));
      }
      if (!stopped) error = "The TV didn't allow this. Reopen TV setup to try again.";
    })();
    return () => {
      stopped = true;
    };
  });
  async function pair() {
    busy = true;
    error = "";
    const target = targetOf(device);
    try {
      await api.query(target, { kind: "pairAgent", code });
      const r = await api.query<{ route: string }>(target, { kind: "verifyAgent" });
      if (r.route === "shell") refresh++;
      else error = "Paired, but Termux can't reach the TV yet. Reopen TV setup to try again.";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
      code = "";
    }
  }
  async function run(step: SetupStep) {
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
</script>

{#snippet helpers()}
  <details class="setup-optional">
    <summary>Shizuku, for other apps</summary>
    <p class="subtle">
      Bridgecoil doesn't need this. Install it if another app asks for Shizuku.
    </p>
    {#if snapshot?.manager && snapshot.manager !== "existing"}
      <p class="subtle" role="status">
        {managerName} is installed{snapshot.serverRunning ? " and running" : ""}.
      </p>
      {#if !snapshot.serverRunning}<button
          class="secondary"
          disabled={busy}
          onclick={() =>
            run({ kind: "startServer", manager: snapshot!.manager! })}
          >Start {managerName}</button
        >{/if}
    {:else}
      {#if snapshot?.helper}<p class="subtle">
          {snapshot.helper.label} is already on this TV.
        </p>{/if}
      {#each managers as manager}<button
          class="secondary"
          disabled={busy ||
            (manager.id === "shizukuNext" &&
              ["other", "unknown"].includes(snapshot?.shizuku ?? ""))}
          onclick={() => install(manager.id)}>Install {manager.name}</button
        >{/each}
    {/if}
  </details>
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
    {#if snapshot.stage === "terminal"}
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
    {:else if snapshot.stage === "pair"}
      {#if !snapshot.agent}<p role="status">Getting Termux ready…</p>
      {:else if approves}
        <p>
          Your TV will ask to allow a connection. Choose
          <strong>Always allow</strong> with your remote.
        </p>
        <p role="status">Waiting for the TV. This moves on by itself.</p>
      {:else}
        <p>
          On the TV, open <strong>Wireless debugging</strong> in Developer
          options, and choose <strong>Pair device with pairing code</strong>.
          Type the code here.
        </p>
        <input
          class="pair-code"
          aria-label="Pairing code"
          inputmode="numeric"
          autocomplete="off"
          maxlength="6"
          bind:value={code}
        />
        <button
          class="primary"
          disabled={busy || !/^\d{6}$/.test(code)}
          onclick={pair}>Connect</button
        >
      {/if}
      {#if snapshot.agent}<p class="subtle">
          Your TV may show a Termux:Boot page for about 2 seconds afterwards.
          That's normal, and it closes by itself.
        </p>{/if}
    {:else if snapshot.stage === "startup"}
      <p>Termux is connected. One step left.</p>
      <button
        class="primary"
        disabled={busy}
        onclick={() => run({ kind: "installMonitor" })}
        >Keep mappings running</button
      >
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
      {@render helpers()}
    {/if}
  </div>
{:else}<p class="error-text" role="alert">{error}</p>{/if}
