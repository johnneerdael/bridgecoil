<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type InstallProgress,
    type Manager,
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
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    onbuttons: () => void;
  } = $props();
  const steps: { id: Stage; title: string }[] = [
    { id: "manager", title: "Choose a helper app" },
    { id: "server", title: "Start it" },
    { id: "terminal", title: "Set up Termux" },
    { id: "shell", title: "Connect Termux" },
    { id: "access", title: "Allow access" },
    { id: "startup", title: "Survive restarts" },
  ];
  const managers: { id: Manager; name: string; note: string }[] = [
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
  const current = $derived(steps.findIndex((s) => s.id === snapshot?.stage));
  const finished = $derived(snapshot?.stage === "done");
  const managerName = $derived(
    managers.find((m) => m.id === snapshot?.manager)?.name ?? "",
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
      .then((r) => {
        if (!cancelled) snapshot = r;
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
  async function checkAccess() {
    busy = true;
    error = "";
    try {
      const { route } = await api.query<{ route: string }>(targetOf(device), {
        kind: "verifyShell",
      });
      if (route === "none")
        error = `Termux hasn't been allowed yet. Choose ${allowLabel} on the TV, then check again.`;
      else refresh++;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const press = (key: string) =>
    api.remote(targetOf(device), { kind: "key", key, longPress: false });
</script>

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
    </p>{/if}
  <div class="setup-card">
    {#if snapshot.stage === "manager"}
      <p>
        These helper apps give your TV the access it needs. Pick one and
        Bridgecoil gets the latest version for you.
      </p>
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
      <button
        class="primary"
        disabled={busy}
        onclick={() => run({ kind: "requestAccess" })}>Ask the TV</button
      >
      <p class="subtle">Can't reach the TV remote? Use these.</p>
      <div class="setup-remote">
        {#each [["left", "Left"], ["ok", "OK"], ["right", "Right"]] as [key, label]}<button
            class="secondary"
            onclick={() => press(key)}>{label}</button
          >{/each}
      </div>
      <button class="secondary" disabled={busy} onclick={checkAccess}
        >Check access</button
      >
    {:else if snapshot.stage === "startup" && snapshot.manager}
      <p>
        When a TV restarts, these background services stop. Do these three
        things so they come back without your phone.
      </p>
      <button
        class="secondary"
        disabled={busy}
        onclick={() => run({ kind: "allowBackground" })}
        >1. Let Termux start with the TV</button
      >
      <button
        class="secondary"
        disabled={busy}
        onclick={() =>
          run({ kind: "allowBootStart", manager: snapshot!.manager! })}
        >2. Open {managerName} on the TV</button
      >
      <p class="subtle">
        In {managerName}, turn on start at boot and pair it with wireless
        debugging. This part happens on the TV.
      </p>
      <button
        class="primary"
        disabled={busy}
        onclick={() => run({ kind: "installMonitor" })}
        >3. Keep mappings running</button
      >
    {:else if snapshot.stage === "done"}
      <h3>Your TV is ready</h3>
      <p>Everything is in place to remap buttons.</p>
      <p class="subtle">
        Recovery after a restart has not been tested on this TV yet. After you
        map a button, restart the TV and try it.
      </p>
      <button class="primary" onclick={onbuttons}>Remap buttons</button>
      <button
        class="secondary"
        disabled={busy}
        onclick={() => run({ kind: "removeMonitor" })}
        >Remove the startup script</button
      >
    {/if}
  </div>
{:else}<p class="error-text" role="alert">{error}</p>{/if}
