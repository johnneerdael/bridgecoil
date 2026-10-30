<script lang="ts">
  import { onMount } from "svelte";
  import Modal from "./lib/Modal.svelte";
  import WorkspaceScreen from "./lib/workspace/WorkspaceScreen.svelte";
  import Icon from "./lib/workspace/Icon.svelte";
  import { destinations, type Destination } from "./lib/workspace/client";
  import {
    parseTheme,
    applyTheme,
    themeKey,
    type ThemePreference,
  } from "./lib/theme/theme";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    canInstall,
    errorMessage,
    type Device,
    type Selection,
  } from "./lib/model";
  const native = isTauri();
  let theme = $state<ThemePreference>(
    parseTheme(localStorage.getItem(themeKey)),
  );
  let systemDark = $state(false);
  $effect(() => {
    const dark = applyTheme(theme, systemDark) === "dark";
    localStorage.setItem(themeKey, theme);
    if (native) void invoke("set_appearance", { dark }).catch(console.error);
  });
  onMount(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!media) return;
    systemDark = media.matches;
    const changed = (event: MediaQueryListEvent) =>
      (systemDark = event.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  });
  let page = $state<Destination>("connect");
  const activeDestination = $derived(
    page === "connect" ? "device" : page === "install" ? "apps" : page,
  );
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
  let permissionDenied = $state(false);
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
    permissionDenied = false;
    try {
      await action();
    } catch (e) {
      error = errorMessage(e);
      permissionDenied =
        !!e && typeof e === "object" && "kind" in e && e.kind === "permission";
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
      notice = "Connected. Your device workspace is ready.";
      page = "device";
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

  let menuOpen = $state(false);
  let helpOpen = $state(false);
  let deviceOpen = $state(false);
  let fileOpen = $state(false);
  let feedbackOpen = $state(false);
  let previewOpen = $state(false);
  let installHelpOpen = $state(false);
  let keyboard = $state(false);
  let messagePage = $state(0);
  const messageText = $derived(error || notice);
  const messagePages = $derived(
    Math.max(1, Math.ceil(messageText.length / 400)),
  );
  function showFeedback() {
    messagePage = 0;
    feedbackOpen = true;
  }
  function navigate(next: Destination) {
    page = next;
    menuOpen = false;
  }
  function focusInput(event: FocusEvent) {
    keyboard =
      event.target instanceof HTMLInputElement &&
      event.target.type !== "checkbox";
  }
  const label = (value: string | undefined, max = 100) =>
    value && value.length > max ? value.slice(0, max) + "…" : value;
</script>

<div
  class="workspace"
  class:keyboard
  onfocusin={focusInput}
  onfocusout={() => (keyboard = false)}
>
  <header class="topbar">
    <button
      class="icon-button menu-toggle"
      onclick={() => (menuOpen = true)}
      aria-label="Open navigation"
      disabled={busy}
      ><svg viewBox="0 0 24 24" aria-hidden="true"
        ><path d="M4 6h16M4 12h16M4 18h16" /></svg
      ></button
    >
    <div class="brand">
      <div class="brand-wordmark" aria-label="Bridgecoil">
        <img
          class="logo-dark"
          src="/brand/bridgecoil-horizontal-dark.svg"
          alt=""
        />
        <img
          class="logo-light"
          src="/brand/bridgecoil-horizontal-light.svg"
          alt=""
        />
      </div>
      <small>{device ? label(device.model, 36) : "Android TV workspace"}</small>
    </div>
    <button
      class="connection-badge"
      class:online={device}
      aria-label={`Connection status: ${device ? "Connected" : "Not connected"}`}
      onclick={() => {
        if (device) deviceOpen = true;
        else page = "connect";
      }}
      disabled={busy}
      ><span class="status-dot" class:online={device}></span>{busy
        ? "Working…"
        : device
          ? "Connected"
          : "Not connected"}</button
    >
  </header>
  <main id="main">
    {#if !native}<button
        class="preview-notice"
        onclick={() => (previewOpen = true)}
        ><strong>Native app required</strong><span>Preview only · details</span
        ></button
      >{/if}
    <header class="page-heading">
      <h1>
        {page === "connect"
          ? "Connect a device"
          : page === "install"
            ? "Install an app"
            : destinations.find((d) => d.id === page)?.label}
      </h1>
      {#if page === "connect" || page === "install"}
        <button
          class="help-trigger"
          aria-label={page === "connect"
            ? "Finding the debugging options"
            : "About APK installation"}
          onclick={() => {
            if (page === "connect") helpOpen = true;
            else installHelpOpen = true;
          }}>Help <span aria-hidden="true">?</span></button
        >
      {/if}
    </header>
    {#if error || notice}<aside
        class="feedback"
        class:failure={error}
        class:success={!error}
        role={error ? "alert" : "status"}
      >
        <div>
          <strong
            >{error
              ? unknown
                ? "Installation outcome unknown"
                : permissionDenied
                  ? "Local network access is off"
                  : "Action needs attention"
              : "Done"}</strong
          >
          <p>{messageText.slice(0, 200)}</p>
        </div>
        <button class="text-action" onclick={showFeedback}>Details</button>
      </aside>{/if}
    <div class="page-content">
      {#if page !== "connect" && page !== "install"}
        <WorkspaceScreen
          {page}
          {device}
          ondevice={(d) => {
            if (JSON.stringify(d) !== JSON.stringify(device)) {
              device = d;
              selection = null;
              confirmed = false;
            }
          }}
          onnavigate={navigate}
        />
      {:else if page === "connect"}
        {#if device}
          <section class="center-state">
            <div class="state-icon" aria-hidden="true">✓</div>
            <h2>Connection ready</h2>
            <p>{label(device.model, 60)}</p>
            <p class="subtle">
              Android {device.androidVersion} · {device.encrypted
                ? "TLS connection"
                : "Legacy · unencrypted"}
            </p>
            <button
              class="primary"
              onclick={() => (page = "install")}
              disabled={busy}
              >Install an app <span aria-hidden="true">→</span></button
            ><button
              class="secondary"
              onclick={() => (deviceOpen = true)}
              disabled={busy}>Connection details</button
            >
          </section>
        {:else}
          <section class="connection-flow">
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
              <p class="connection-hint">
                Open <strong>Pair device with pairing code</strong> in your TV’s Wireless
                debugging settings.
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
                />
                <label for="pair-code">Six-digit pairing code</label><input
                  id="pair-code"
                  class="code-input"
                  bind:value={code}
                  placeholder="000000"
                  inputmode="numeric"
                  pattern="[0-9]{6}"
                  maxlength="6"
                  autocomplete="off"
                  disabled={busy}
                />
                <button
                  class="primary"
                  type="submit"
                  disabled={!native ||
                    busy ||
                    !pairEndpoint.trim() ||
                    !/^[0-9]{6}$/.test(code)}
                  >{busy ? busyLabel : "Pair TV"}<span aria-hidden="true"
                    >→</span
                  ></button
                >
              </form>
              <p class="field-note">
                Keep the TV’s pairing screen open. Already paired? Choose
                Connect above.
              </p>
            {:else}
              <p class="connection-hint">
                Use the address on the <strong
                  >main Wireless debugging screen</strong
                >. Its port differs from the pairing port.
              </p>
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  void connect();
                }}
              >
                <label for="connect-address"
                  >Connection address <span
                    >IP or hostname : connection port</span
                  ></label
                ><input
                  id="connect-address"
                  bind:value={endpoint}
                  placeholder="192.168.1.42:40215"
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  disabled={busy}
                />
                <label class="checkbox"
                  ><input
                    type="checkbox"
                    bind:checked={legacy}
                    disabled={busy}
                  /><span
                    >Allow legacy, unencrypted ADB<small
                      >For an already enabled TV on a trusted local network.</small
                    ></span
                  ></label
                >
                <button
                  class="primary"
                  type="submit"
                  disabled={!native || busy || !endpoint.trim()}
                  >{busy ? busyLabel : "Connect to TV"}<span aria-hidden="true"
                    >→</span
                  ></button
                >
              </form>
            {/if}
          </section>
        {/if}
      {:else}
        {#if !device}
          <section class="center-state">
            <div class="state-icon" aria-hidden="true">⇄</div>
            <h2>Connect your device first</h2>
            <p class="subtle">Choose where the app will be installed.</p>
            <button class="primary" onclick={() => (page = "connect")}
              >Set up connection <span aria-hidden="true">→</span></button
            >
          </section>
        {:else if selection}
          {#if busy && progress}
            <section class="center-state">
              <div class="state-icon" aria-hidden="true">↑</div>
              <h2>
                {progress.stage === "uploading"
                  ? "Transferring APK"
                  : progress.stage === "installing"
                    ? "Installing app"
                    : "Verifying installation"}
              </h2>
              <p>{label(selection.package, 80)}</p>
              <div class="transfer">
                <progress max={progress.total || 1} value={progress.sent}
                ></progress><span
                  >{formatBytes(progress.sent)} / {formatBytes(
                    progress.total,
                  )}</span
                >
              </div>
              <p class="subtle">Keep the app open and the device connected.</p>
            </section>
          {:else}
            <section class="install-review">
              <div class="apk-heading">
                <span class="file-icon" aria-hidden="true">↓</span>
                <div>
                  <h2>{label(selection.package, 90)}</h2>
                  <p>
                    {label(
                      selection.versionName ??
                        `Version ${selection.versionCode}`,
                      60,
                    )} · {formatBytes(selection.bytes)}
                  </p>
                </div>
              </div>
              <p class="target-caption">On {label(device.model, 45)} · Owner</p>
              <button
                class="text-action file-details"
                onclick={() => (fileOpen = true)}
                >File details & fingerprint <span aria-hidden="true">↗</span
                ></button
              >
              <p class="install-note">
                Android checks update compatibility. No app is removed
                automatically.
              </p>
              <label class="checkbox"
                ><input
                  type="checkbox"
                  bind:checked={confirmed}
                  disabled={busy}
                /><span
                  >Install this APK on this device<small
                    >I chose and trust the file source.</small
                  ></span
                ></label
              >
              <div class="install-actions">
                <button class="primary" onclick={install} disabled={!ready}
                  >{busy ? busyLabel : "Install app"}<span aria-hidden="true"
                    >↑</span
                  ></button
                ><button
                  class="secondary"
                  aria-label="Choose another APK"
                  onclick={pick}
                  disabled={busy || !native}>Change file</button
                >
              </div>
            </section>
          {/if}
        {:else}
          <section class="center-state">
            <div class="state-icon" aria-hidden="true">⇧</div>
            <h2>Choose your APK</h2>
            <p class="subtle">
              Select a standalone APK using the native file picker.
            </p>
            <p class="target-caption">
              Target: {label(device.model, 55)} · Profile {device.profileId}
            </p>
            <button class="primary" onclick={pick} disabled={busy || !native}
              >{busy ? busyLabel : "Choose APK file"}<span aria-hidden="true"
                >＋</span
              ></button
            >
          </section>
        {/if}
      {/if}
    </div>
  </main>
</div>

<Modal
  open={menuOpen}
  title="Navigation"
  variant="drawer"
  onclose={() => (menuOpen = false)}
>
  <nav class="drawer-menu" aria-label="Main navigation">
    {#each destinations as destination}<button
        class:active={activeDestination === destination.id}
        aria-label={destination.label}
        aria-current={activeDestination === destination.id ? "page" : undefined}
        onclick={() => navigate(destination.id)}
        ><Icon name={destination.id} /><span
          >{destination.label}<small>{destination.note}</small></span
        ></button
      >{/each}
  </nav>
  <fieldset class="theme-picker">
    <legend>Appearance</legend>
    <div>
      {#each ["system", "light", "dark"] as choice}
        <button
          class:chosen={theme === choice}
          aria-pressed={theme === choice}
          onclick={() => (theme = choice as ThemePreference)}
          >{choice[0].toUpperCase() + choice.slice(1)}</button
        >
      {/each}
    </div>
  </fieldset>
</Modal>
<Modal
  open={helpOpen}
  title="Finding the debugging options"
  onclose={() => (helpOpen = false)}
>
  <p>
    On most TVs, enable Developer options by repeatedly selecting Android TV OS
    build under Settings → System → About. Menu names vary. Both devices need
    local network access.
  </p>
  <p>
    If your TV only provides older network debugging, enable it on the TV first,
    then use Connect with the legacy option. This app cannot enable debugging on
    an unauthorised device.
  </p>
  <p>
    This version uses manual addresses. Copy the pairing and connection ports
    from their separate TV screens.
  </p>
</Modal>
<Modal
  open={deviceOpen}
  title="Device connection"
  onclose={() => (deviceOpen = false)}
>
  {#if device}<h3>{label(device.model, 120)}</h3>
    <dl>
      <div>
        <dt>Address</dt>
        <dd>{device.endpoint}</dd>
      </div>
      <div>
        <dt>Transport</dt>
        <dd>{device.encrypted ? "TLS" : "Legacy · unencrypted"}</dd>
      </div>
      <div>
        <dt>Android</dt>
        <dd>{device.androidVersion} · SDK {device.sdk || "unknown"}</dd>
      </div>
      <div>
        <dt>Profile</dt>
        <dd>Profile {device.profileId}</dd>
      </div>
    </dl>
    <button
      class="secondary"
      onclick={() => {
        deviceOpen = false;
        void disconnect();
      }}
      disabled={busy}>Disconnect</button
    >{/if}
</Modal>
<Modal
  open={fileOpen}
  title="APK file details"
  onclose={() => (fileOpen = false)}
>
  {#if selection}<dl>
      <div>
        <dt>Package</dt>
        <dd>{label(selection.package, 255)}</dd>
      </div>
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
            ? selection.abis.slice(0, 4).join(", ")
            : "No native-library restriction"}
        </dd>
      </div>
      <div>
        <dt>Size</dt>
        <dd>{formatBytes(selection.bytes)}</dd>
      </div>
    </dl>
    <h3>SHA-256 fingerprint</h3>
    <code class="fingerprint">{selection.sha256}</code>
    <p class="subtle">
      Metadata inspection does not authenticate the APK publisher.
    </p>{/if}
</Modal>
<Modal
  open={feedbackOpen}
  title={error ? "Action details" : "Completed action"}
  onclose={() => (feedbackOpen = false)}
>
  <p class="message-detail">
    {messageText.slice(messagePage * 400, (messagePage + 1) * 400)}
  </p>
  {#if unknown}<p>
      Reconnect and inspect the installed app before retrying.
    </p>{/if}
  {#if messagePages > 1}<div class="pagination">
      <button
        class="secondary"
        disabled={messagePage === 0}
        onclick={() => messagePage--}>Previous</button
      ><span>{messagePage + 1} / {messagePages}</span><button
        class="secondary"
        disabled={messagePage + 1 >= messagePages}
        onclick={() => messagePage++}>Next</button
      >
    </div>{/if}
</Modal>
<Modal
  open={previewOpen}
  title="Native app required"
  onclose={() => (previewOpen = false)}
  ><p>
    This is the interface preview. Run the installed Tauri app to pair, connect
    and choose an APK with the native file picker. No device results are
    simulated here.
  </p></Modal
>
<Modal
  open={installHelpOpen}
  title="About APK installation"
  onclose={() => (installHelpOpen = false)}
  ><p>
    Choose a standalone APK, inspect the package and target device, then confirm
    installation. Android checks signing and update compatibility.
  </p>
  <p>
    Existing apps are never automatically uninstalled. Split APK sets and
    Android App Bundles are not supported by this build. The current file limit
    is 1 GiB.
  </p></Modal
>
