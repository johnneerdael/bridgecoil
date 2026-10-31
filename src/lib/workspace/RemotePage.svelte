<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Platform,
    type RemoteAction,
    type Operation,
  } from "./client";
  import Icon from "./Icon.svelte";
  import Modal from "../Modal.svelte";
  let {
    device,
    api,
    onresult,
    onapps,
    onbuttons,
  }: {
    device: Device;
    api: Platform;
    onresult: (r: Operation) => void;
    onapps: () => void;
    onbuttons?: () => void;
  } = $props();
  let busy = $state(false);
  let message = $state("");
  let textOpen = $state(false);
  let text = $state("");
  let longPress = $state(false);
  async function send(action: RemoteAction) {
    if (busy) return;
    busy = true;
    message = "";
    const id = device.id;
    try {
      const r = await api.remote(targetOf(device), action);
      if (device.id === id)
        message =
          r.status === "sent"
            ? "Sent · check the TV"
            : r.error || r.output || r.status;
      onresult(r);
    } catch (e) {
      message = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const key = (key: string) => send({ kind: "key", key, longPress });
</script>

<section class="feature-page remote-page">
  <div class="toolbar">
    <button class="secondary" onclick={() => (textOpen = true)}
      >Type on TV</button
    ><button class="secondary" onclick={onapps}>Open an app</button>
  </div>
  {#if onbuttons}<button class="text-action" onclick={onbuttons}
      >Remap your remote's buttons</button
    >{/if}
  <div class="dpad" aria-label="Directional remote">
    <button class="up" aria-label="Up" disabled={busy} onclick={() => key("up")}
      ><Icon name="arrow-up" /></button
    ><button
      class="left"
      aria-label="Left"
      disabled={busy}
      onclick={() => key("left")}><Icon name="arrow-left" /></button
    ><button
      class="ok"
      aria-label="Select"
      disabled={busy}
      onclick={() => key("ok")}>OK</button
    ><button
      class="right"
      aria-label="Right"
      disabled={busy}
      onclick={() => key("right")}><Icon name="arrow-right" /></button
    ><button
      class="down"
      aria-label="Down"
      disabled={busy}
      onclick={() => key("down")}><Icon name="arrow-down" /></button
    >
  </div>
  <div class="remote-row">
    {#each [["back", "Back", "arrow-left"], ["home", "Home", "home"], ["menu", "Menu", "menu"]] as [id, label, icon]}<button
        disabled={busy}
        onclick={() => key(id)}><Icon name={icon} /><span>{label}</span></button
      >{/each}
  </div>
  <div class="remote-row">
    {#each [["previous", "Previous"], ["playPause", "Play / pause"], ["next", "Next"]] as [id, label]}<button
        disabled={busy}
        onclick={() => key(id)}>{label}</button
      >{/each}
  </div>
  <div class="remote-row">
    {#each [["volumeDown", "Volume −"], ["mute", "Mute"], ["volumeUp", "Volume +"]] as [id, label]}<button
        disabled={busy}
        onclick={() => key(id)}>{label}</button
      >{/each}
  </div>
  <label class="checkbox"
    ><input type="checkbox" bind:checked={longPress} /><span
      >Send as long press</span
    ></label
  >
  <p class="subtle" role="status">
    {busy
      ? "Sending…"
      : message ||
        "Commands use the active connection. Sent does not confirm the TV's response."}
  </p>
</section>
<Modal open={textOpen} title="Type on TV" onclose={() => (textOpen = false)}
  ><p class="subtle">
    Focus a text field on the TV first. This route supports printable ASCII,
    excluding %. Use the TV keyboard for other characters.
  </p>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void send({ kind: "text", text });
    }}
  >
    <label for="remote-text">Text</label><input
      id="remote-text"
      bind:value={text}
      maxlength={1024}
    /><button class="primary" disabled={busy || !text} type="submit"
      >Send text</button
    >
  </form>
  {#if textOpen}<p class="subtle" role="status">{message}</p>{/if}</Modal
>
