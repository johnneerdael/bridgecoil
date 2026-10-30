<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    canInstall,
    errorMessage,
    type Device,
    type Selection,
  } from "./lib/model";
  const native = isTauri();
  let page = $state<"connect" | "install">("connect");
  let step = $state<"pair" | "connect">("pair");
  let pairEndpoint = $state("");
  let code = $state("");
  let endpoint = $state("");
  let legacy = $state(false);
  let device = $state<Device | null>(null);
  let selection = $state<Selection | null>(null);
  let busy = $state(false);
  let busyLabel = $state("");
  let error = $state("");
  let notice = $state("");
  let unknown = $state(false);
  let progress = $state<{
    stage: string;
    sent: number;
    total: number;
    selectionId: string;
  } | null>(null);
  let confirmed = $state(false);
  const ready = $derived(
    native && canInstall(device, selection, busy) && confirmed,
  );
  const formatBytes = (n: number) =>
    n >= 1048576
      ? `${(n / 1048576).toFixed(1)} MB`
      : `${Math.round(n / 1024)} KB`;
  onMount(() => {
    let dispose: UnlistenFn | undefined;
    let stopped = false;
    endpoint = localStorage.getItem("adb.connectionEndpoint") ?? "";
    if (native) {
      void invoke<Device | null>("connection_status")
        .then((v) => {
          if (!stopped) device = v;
        })
        .catch((e) => {
          if (!stopped) error = errorMessage(e);
        });
      void listen<NonNullable<typeof progress>>("install-progress", (e) => {
        if (e.payload.selectionId === selection?.id) progress = e.payload;
      })
        .then((fn) => {
          if (stopped) fn();
          else dispose = fn;
        })
        .catch((e) => {
          if (!stopped) error = errorMessage(e);
        });
    }
    return () => {
      stopped = true;
      dispose?.();
    };
  });
  async function run(label: string, action: () => Promise<void>) {
    if (busy || !native) return;
    busy = true;
    busyLabel = label;
    error = "";
    notice = "";
    unknown = false;
    try {
      await action();
    } catch (e) {
      error = errorMessage(e);
      unknown =
        !!e &&
        typeof e === "object" &&
        "kind" in e &&
        e.kind === "unknownInstall";
      try {
        device = await invoke<Device | null>("connection_status");
      } catch {
        device = null;
      }
    } finally {
      busy = false;
      busyLabel = "";
    }
  }
  async function pair() {
    await run("Pairing with TV", async () => {
      await invoke<string>("pair_device", { endpoint: pairEndpoint, code });
      code = "";
      step = "connect";
      notice =
        "Paired. Enter the connection address from the TV’s main Wireless debugging screen.";
    });
  }
  async function connect() {
    selection = null;
    confirmed = false;
    device = null;
    await run("Connecting to TV", async () => {
      device = await invoke<Device>("connect_device", {
        endpoint,
        allowLegacy: legacy,
      });
      localStorage.setItem("adb.connectionEndpoint", endpoint.trim());
      notice = "Connected. Choose an APK to install.";
      page = "install";
    });
  }
  async function disconnect() {
    await run("Disconnecting", async () => {
      await invoke("disconnect_device");
      device = null;
      selection = null;
      confirmed = false;
      progress = null;
      page = "connect";
    });
  }
  async function pick() {
    if (!device) return;
    confirmed = false;
    selection = null;
    progress = null;
    const sessionId = device.id;
    await run("Reading the selected APK", async () => {
      selection = await invoke<Selection | null>("choose_apk", { sessionId });
    });
  }
  async function install() {
    if (!ready || !device || !selection) return;
    const chosen = selection;
    const target = device;
    await run("Installing on TV", async () => {
      await invoke("install_apk", {
        sessionId: target.id,
        selectionId: chosen.id,
      });
      notice = `Verified: ${chosen.package} · version ${chosen.versionCode} is installed on ${target.model}.`;
    });
    selection = null;
    confirmed = false;
    progress = null;
  }
</script>

<div class="workspace">
  <header class="topbar">
    <a class="brand" href="#main" aria-label="ADB Studio"
      ><span class="brand-mark">a›</span><span
        >ADB Studio<small>CONNECTION & INSTALL POC</small></span
      ></a
    ><span class="build-tag">Early build</span>
  </header>
  <div class="devicebar">
    <span class="tv" aria-hidden="true">▣</span>
    <div>
      <strong>{device?.model ?? "No TV connected"}</strong><small
        >{device
          ? `${device.endpoint} · ${device.encrypted ? "TLS connection" : "Legacy · unencrypted"}`
          : "Your TV, connected directly."}</small
      >
    </div>
    <span
      class:online={device}
      class="status-dot"
      aria-label={device ? "Connected" : "Disconnected"}
    ></span>
  </div>
  <main id="main">
    {#if !native}<aside class="message info">
        <strong>Native app required</strong>
        <p>
          This is the interface preview. Run the Tauri app to pair, connect and
          choose an APK using your native file picker. No device results are
          simulated here.
        </p>
      </aside>{/if}
    {#if error}<aside class="message failure" role="alert">
        <strong
          >{unknown
            ? "Installation outcome unknown"
            : "Could not complete the action"}</strong
        >
        <p>{error}</p>
        {#if unknown}<p>
            Reconnect and check the installed app on your TV before trying
            again.
          </p>{/if}
      </aside>{/if}
    {#if notice}<aside class="message success" role="status">
        <p>{notice}</p>
      </aside>{/if}
    {#if page === "connect"}
      <div class="eyebrow">01 / CONNECT</div>
      <h1>Bring your TV<br /> into reach.</h1>
      <p class="intro">
        Pair once. Connect directly. Install the apps you choose.
      </p>
      {#if device}
        <section class="connected-panel">
          <div class="large-icon" aria-hidden="true">▣</div>
          <h2>{device.model}</h2>
          <p>Android {device.androidVersion} · SDK {device.sdk || "unknown"}</p>
          <p class="subtle">{device.abis.join(" · ")}</p>
          <button
            class="primary"
            onclick={() => (page = "install")}
            disabled={busy}
            >Install an app <span aria-hidden="true">→</span></button
          ><button class="secondary" onclick={disconnect} disabled={busy}
            >Disconnect</button
          >
        </section>
      {:else}
        <div class="steps" role="group" aria-label="Connection step">
          <button
            class:chosen={step === "pair"}
            aria-pressed={step === "pair"}
            onclick={() => (step = "pair")}
            disabled={busy}><span>1</span> Pair TV</button
          ><button
            class:chosen={step === "connect"}
            aria-pressed={step === "connect"}
            onclick={() => (step = "connect")}
            disabled={busy}><span>2</span> Connect</button
          >
        </div>
        {#if step === "pair"}
          <section class="form-section">
            <h2>Pair with a code</h2>
            <p class="subtle">
              On the TV: Developer options → Wireless debugging → Pair device
              with pairing code. Keep that screen open.
            </p>
            <form
              onsubmit={(e) => {
                e.preventDefault();
                void pair();
              }}
            >
              <label for="pair-address"
                >Pairing address <span>IP or hostname : pairing port</span
                ></label
              ><input
                id="pair-address"
                bind:value={pairEndpoint}
                placeholder="192.168.1.42:37123"
                autocomplete="off"
                autocapitalize="off"
                spellcheck="false"
                disabled={busy}
              /><label for="pair-code">Six-digit pairing code</label><input
                class="code-input"
                id="pair-code"
                bind:value={code}
                placeholder="000000"
                inputmode="numeric"
                pattern="[0-9]{6}"
                maxlength="6"
                autocomplete="off"
                disabled={busy}
              /><button
                class="primary"
                type="submit"
                disabled={!native ||
                  busy ||
                  !pairEndpoint.trim() ||
                  !/^[0-9]{6}$/.test(code)}
              >
                {busy ? busyLabel : "Pair TV"}
                <span aria-hidden="true">→</span></button
              >
            </form>
            <button
              class="text-action"
              onclick={() => (step = "connect")}
              disabled={busy}
              >Already paired, or using legacy network debugging?</button
            >
          </section>
        {:else}
          <section class="form-section">
            <h2>Connect to your TV</h2>
            <p class="subtle">
              Use the address on the TV’s main Wireless debugging screen. Its
              port is different from the pairing port.
            </p>
            <form
              onsubmit={(e) => {
                e.preventDefault();
                void connect();
              }}
            >
              <label for="connect-address"
                >Connection address <span>IP or hostname : connection port</span
                ></label
              ><input
                id="connect-address"
                bind:value={endpoint}
                placeholder="192.168.1.42:40215"
                autocomplete="off"
                autocapitalize="off"
                spellcheck="false"
                disabled={busy}
              /><label class="checkbox"
                ><input
                  type="checkbox"
                  bind:checked={legacy}
                  disabled={busy}
                /><span
                  >Allow legacy, unencrypted ADB<small
                    >Only for an already enabled TV on a trusted local network.
                    You may need to approve access on the TV.</small
                  ></span
                ></label
              ><button
                class="primary"
                type="submit"
                disabled={!native || busy || !endpoint.trim()}
                >{busy ? busyLabel : "Connect to TV"}<span aria-hidden="true"
                  >→</span
                ></button
              >
            </form>
          </section>
        {/if}
        <details class="help">
          <summary>Finding the debugging options</summary>
          <p>
            On most TVs, enable Developer options by repeatedly selecting
            Android TV OS build under Settings → System → About. Menu names
            vary. Both devices need local network access.
          </p>
          <p>
            If your TV only provides older network debugging, enable it on the
            TV first, then use Connect with the legacy option. This app cannot
            enable debugging on an unauthorised device.
          </p>
          <p>
            This POC uses manual addresses. Automatic discovery will follow.
          </p>
        </details>
      {/if}
    {:else}
      <div class="eyebrow">02 / INSTALL</div>
      <h1>A new app.<br /> On your TV.</h1>
      <p class="intro">
        Choose a standalone APK. Review it before anything changes.
      </p>
      {#if !device}<section class="empty-state">
          <div class="large-icon" aria-hidden="true">⇄</div>
          <h2>Connect your TV first</h2>
          <p class="subtle">
            Pair and connect to choose where the app is installed.
          </p>
          <button class="primary" onclick={() => (page = "connect")}
            >Set up connection <span aria-hidden="true">→</span></button
          >
        </section>
      {:else}
        <div class="target-line">
          <span class="status-dot online"></span><span
            >{device.model}<small>Install for Owner · Android user 0</small
            ></span
          ><button
            class="text-action"
            onclick={() => (page = "connect")}
            disabled={busy}>Change</button
          >
        </div>
        {#if selection}
          <section class="apk-review">
            <div class="apk-heading">
              <span class="file-icon" aria-hidden="true">↓</span>
              <div>
                <h2>{selection.package}</h2>
                <p>
                  {selection.versionName ?? `Version ${selection.versionCode}`} ·
                  {formatBytes(selection.bytes)}
                </p>
              </div>
            </div>
            <dl>
              <div>
                <dt>Version code</dt>
                <dd>{selection.versionCode}</dd>
              </div>
              <div>
                <dt>Minimum SDK</dt>
                <dd>{selection.minSdk ?? "Not specified"}</dd>
              </div>
              <div>
                <dt>Architecture</dt>
                <dd>
                  {selection.abis.length
                    ? selection.abis.join(", ")
                    : "No native-library restriction"}
                </dd>
              </div>
              <div>
                <dt>Target</dt>
                <dd>{device.model} · Owner</dd>
              </div>
            </dl>
            <details class="technical">
              <summary>File fingerprint</summary><code>{selection.sha256}</code>
              <p>
                Metadata is read from the selected file. This POC does not
                authenticate the APK’s publisher.
              </p>
            </details>
            <div class="message info">
              <p>
                Android checks signing and update compatibility. An existing app
                is updated in place when compatible; this POC will never
                uninstall it automatically.
              </p>
            </div>
            <label class="checkbox"
              ><input
                type="checkbox"
                bind:checked={confirmed}
                disabled={busy}
              /><span
                >Install this APK on {device.model}<small
                  >I chose and trust the source of this file.</small
                ></span
              ></label
            >
            {#if progress}<div class="transfer" role="status">
                <div>
                  <strong
                    >{progress.stage === "uploading"
                      ? "Transferring APK"
                      : progress.stage === "installing"
                        ? "Installing on TV"
                        : "Verifying installed version"}</strong
                  ><span
                    >{formatBytes(progress.sent)} / {formatBytes(
                      progress.total,
                    )}</span
                  >
                </div>
                <progress max={progress.total || 1} value={progress.sent}
                ></progress>
                <p class="subtle">
                  Keep the app open and the TV connected. An interruption may
                  leave the result unknown.
                </p>
              </div>{/if}
            <button class="primary" onclick={install} disabled={!ready}
              >{busy ? busyLabel : "Install on TV"}<span aria-hidden="true"
                >↑</span
              ></button
            ><button class="secondary" onclick={pick} disabled={busy || !native}
              >Choose another APK</button
            >
          </section>
        {:else}
          <section class="choose-file">
            <div class="large-icon" aria-hidden="true">⇧</div>
            <h2>Choose your APK</h2>
            <p class="subtle">
              Use your device’s file picker.<br />The file is inspected before
              installation.
            </p>
            <button class="primary" onclick={pick} disabled={busy || !native}
              >{busy ? busyLabel : "Choose APK file"}<span aria-hidden="true"
                >＋</span
              ></button
            >
          </section>
        {/if}
        <p class="footnote">
          Standalone APKs up to 1 GiB. Split APK sets and Android App Bundles
          are outside this first POC. Unsupported files are rejected.
        </p>
      {/if}
    {/if}
  </main>
  <nav class="bottom-nav" aria-label="Main navigation">
    <button
      class:active={page === "connect"}
      aria-current={page === "connect" ? "page" : undefined}
      onclick={() => (page = "connect")}
      disabled={busy}
      ><svg viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="3" y="4" width="18" height="13" rx="2" /><path
          d="M8 21h8M12 17v4"
        /></svg
      >Connect</button
    ><button
      class:active={page === "install"}
      aria-current={page === "install" ? "page" : undefined}
      onclick={() => (page = "install")}
      disabled={busy}
      ><svg viewBox="0 0 24 24" aria-hidden="true"
        ><path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5" /></svg
      >Install APK</button
    >
  </nav>
</div>
