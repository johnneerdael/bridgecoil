<script lang="ts">
  import { errorMessage, type Device } from "../model";
  import { targetOf, type Platform } from "./client";
  let { device, api }: { device: Device; api: Platform } = $props();
  let image = $state("");
  let busy = $state(false);
  let error = $state("");
  async function take() {
    busy = true;
    error = "";
    try {
      const r = await api.query<{ png: string }>(targetOf(device), {
        kind: "screenshot",
      });
      image = r.png;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
<p class="subtle">
  Shows what's on the TV right now. Streaming apps usually appear black, because
  they protect their video.
</p>
<button class="primary" disabled={busy} onclick={take}
  >{busy ? "Capturing…" : "Take a screenshot"}</button
>
{#if image}<img
    class="tv-screenshot"
    src={`data:image/png;base64,${image}`}
    alt={`Screen of ${device.model}`}
  />{/if}
