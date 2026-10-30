<script lang="ts">
  import type { Device } from "../model";
  import {
    nativePlatform,
    type Destination,
    type Operation,
    type Review,
  } from "./client";
  import DevicePage from "./DevicePage.svelte";
  import AppsPage from "./AppsPage.svelte";
  import RemotePage from "./RemotePage.svelte";
  import ToolsPage from "./ToolsPage.svelte";
  import HistoryPage from "./HistoryPage.svelte";
  import ActionReview from "./ActionReview.svelte";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  let {
    page,
    device,
    ondevice,
    onnavigate,
  }: {
    page: Destination;
    device: Device | null;
    ondevice: (d: Device | null) => void;
    onnavigate: (p: Destination) => void;
  } = $props();
  const api = nativePlatform((d) => ondevice(d));
  let review = $state<Review | null>(null);
  let outcome = $state<Operation | null>(null);
  let reload = $state(0);
  let appFilter = $state("all"),
    appSearch = $state(""),
    appSubmitted = $state(""),
    appPage = $state(0);
  let originalSession = $state("");
  $effect(() => {
    const session = device?.id ?? "";
    if (session !== originalSession) {
      originalSession = session;
      appFilter = "all";
      appSearch = "";
      appSubmitted = "";
      appPage = 0;
      if (review) void api.discard(review.id).catch(() => {});
      review = null;
      outcome = null;
    }
  });
  function close() {
    if (review) void api.discard(review.id).catch(() => {});
    review = null;
  }
  function result(r: Operation) {
    outcome = r;
    reload++;
  }
</script>

{#if page === "history"}{#key reload}<HistoryPage
      {api}
      onreview={(r) => (review = r)}
    />{/key}
{:else if !device}<section class="center-state">
    <h2>Connect your device</h2>
    <p class="subtle">
      Pair or connect a TV to use {page}. Existing local history remains
      available.
    </p>
    <button class="primary" onclick={() => onnavigate("connect")}
      >Open connection setup</button
    >
  </section>
{:else}{#key `${device.id}:${reload}`}
    {#if page === "device"}<DevicePage
        {device}
        {api}
        ondevice={(d) => ondevice(d)}
        {onnavigate}
      />
    {:else if page === "apps"}<AppsPage
        {device}
        {api}
        bind:filter={appFilter}
        bind:search={appSearch}
        bind:submitted={appSubmitted}
        bind:page={appPage}
        onreview={(r) => (review = r)}
        oninstall={() => onnavigate("install")}
      />
    {:else if page === "remote"}<RemotePage
        {device}
        {api}
        onresult={(r) => {
          if (r.status !== "sent") outcome = r;
        }}
        onapps={() => onnavigate("apps")}
      />
    {:else if page === "tools"}<ToolsPage
        {device}
        {api}
        onreview={(r) => (review = r)}
        oninstall={() => onnavigate("install")}
      />{/if}
  {/key}{/if}
<ActionReview
  {review}
  {api}
  deviceName={device?.model ?? "Original device"}
  onclose={close}
  onresult={result}
/>
<Modal
  open={!!outcome}
  title="Operation result"
  onclose={() => (outcome = null)}
  >{#if outcome}<h3>{outcome.title}</h3>
    <p class={`outcome ${outcome.status}`}>{outcome.status}</p>
    <Output
      text={outcome.error ||
        outcome.output ||
        (outcome.status === "sent"
          ? "Sent to the TV. Check its response."
          : "No output returned.")}
    /><button
      class="secondary"
      onclick={() => {
        outcome = null;
        onnavigate("history");
      }}>View history</button
    >{/if}</Modal
>
