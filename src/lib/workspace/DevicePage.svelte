<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Platform,
    type Profile,
    type Destination,
  } from "./client";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  import Icon from "./Icon.svelte";
  let {
    device,
    api,
    onnavigate,
    ondevice,
  }: {
    device: Device;
    api: Platform;
    onnavigate: (p: Destination) => void;
    ondevice: (d: Device) => void;
  } = $props();
  let profiles = $state<Profile[]>([]);
  let snapshot = $state<{
    shellIdentity: string;
    storage: string;
    bootId: string;
    uptimeSeconds: string;
  } | null>(null);
  let error = $state("");
  let busy = $state(false);
  let sheet = $state<"" | "profiles" | "details">("");
  let reload = $state(0);
  $effect(() => {
    const target = targetOf(device);
    reload;
    let cancelled = false;
    busy = true;
    error = "";
    snapshot = null;
    profiles = [];
    void Promise.allSettled([
      api.query<Profile[]>(target, { kind: "profiles" }),
      api.query<NonNullable<typeof snapshot>>(target, { kind: "device" }),
    ]).then(([p, s]) => {
      if (cancelled) return;
      if (p.status === "fulfilled") profiles = p.value;
      else error = errorMessage(p.reason);
      if (s.status === "fulfilled") snapshot = s.value;
      else error = errorMessage(s.reason);
      busy = false;
    });
    return () => {
      cancelled = true;
    };
  });
  async function choose(id: number) {
    busy = true;
    error = "";
    try {
      ondevice(await api.profile(targetOf(device), id));
      sheet = "";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="feature-page">
  <div class="device-card">
    <Icon name="device" size={32} />
    <div>
      <h2>{device.model}</h2>
      <p>{device.manufacturer} · Android {device.androidVersion}</p>
      <small
        >{device.encrypted
          ? "Wireless ADB · TLS"
          : "Legacy ADB · unencrypted"}</small
      >
    </div>
  </div>
  <div class="info-grid">
    <button onclick={() => (sheet = "profiles")}
      ><strong
        >{profiles.find((p) => p.id === device.profileId)?.name ??
          `Profile ${device.profileId}`}</strong
      ><span>Active target profile</span></button
    ><button onclick={() => (sheet = "details")}
      ><strong>Device details</strong><span>Identity, storage and shell</span
      ></button
    >
  </div>
  {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  <div class="tool-grid">
    <button class="tool-tile" onclick={() => onnavigate("apps")}
      ><Icon name="apps" /><strong>Manage apps</strong><small
        >Inspect and review changes</small
      ></button
    >
    <button class="tool-tile" onclick={() => onnavigate("remote")}
      ><Icon name="remote" /><strong>Open remote</strong><small
        >Navigate and launch apps</small
      ></button
    >
    <button class="tool-tile" onclick={() => onnavigate("tools")}
      ><Icon name="tools" /><strong>Device tools</strong><small
        >Launcher, setup and diagnostics</small
      ></button
    >
    <button class="tool-tile" onclick={() => onnavigate("install")}
      ><Icon name="install" /><strong>Install an APK</strong><small
        >Choose a file on this phone</small
      ></button
    >
  </div>
  <div class="page-footer">
    <button class="secondary" disabled={busy} onclick={() => reload++}
      >{busy ? "Inspecting…" : "Refresh"}</button
    ><button class="secondary" onclick={() => onnavigate("connect")}
      >Connection</button
    >
  </div>
</section>
<Modal
  open={sheet === "profiles"}
  title="Target profile"
  onclose={() => (sheet = "")}
>
  <p class="subtle">
    Selecting a profile changes where supported app actions apply. It does not
    switch the TV's foreground user. Old reviews and selected APKs expire.
  </p>
  {#each profiles as profile}<button
      class="list-row"
      disabled={busy}
      onclick={() => choose(profile.id)}
      ><span
        >{profile.name}<small
          >User {profile.id} · {profile.running
            ? "Running"
            : "Not running"}</small
        ></span
      ><span>{profile.id === device.profileId ? "✓" : "Select"}</span></button
    >{/each}
</Modal>
<Modal
  open={sheet === "details"}
  title="Device details"
  onclose={() => (sheet = "")}
>
  <Output
    text={`Model: ${device.model}\nManufacturer: ${device.manufacturer}\nAndroid ${device.androidVersion} · SDK ${device.sdk}\nABI: ${device.abis.join(", ")}\nAddress: ${device.endpoint}\nSerial: ${device.serial || "Not reported"}\nBuild: ${device.build || "Not reported"}\n\nShell: ${snapshot?.shellIdentity ?? "Not inspected"}\n\nStorage:\n${snapshot?.storage ?? "Not inspected"}\n\nBoot ID: ${snapshot?.bootId ?? "Unknown"}\nUptime (seconds): ${snapshot?.uptimeSeconds ?? "Unknown"}`}
  />
</Modal>
