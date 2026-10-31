<script lang="ts">
  import type { Device } from "../model";
  import {
    nativePlatform,
    targetOf,
    sameTarget,
    type BulkReview,
    type Destination,
    type Operation,
    type ButtonFollowUp,
    type Review,
  } from "./client";
  import DevicePage from "./DevicePage.svelte";
  import AppsPage from "./AppsPage.svelte";
  import RemotePage from "./RemotePage.svelte";
  import ToolsPage from "./ToolsPage.svelte";
  import HistoryPage from "./HistoryPage.svelte";
  import ActionReview from "./ActionReview.svelte";
  import BulkActionReview from "./BulkReview.svelte";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  import { resultText } from "./output";
  import { internalStorage } from "./files";
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
  let bulkReview = $state<BulkReview | null>(null);
  let outcome = $state<Operation | null>(null);
  let reload = $state(0);
  let appFilter = $state("all"),
    appSearch = $state(""),
    appSubmitted = $state(""),
    toolChoice = $state(""),
    buttonFollowUp = $state<ButtonFollowUp | null>(null),
    toolFilePath = $state(internalStorage);
  let originalSession = $state("");
  $effect(() => {
    const session = device?.id ?? "";
    if (session !== originalSession) {
      originalSession = session;
      appFilter = "all";
      appSearch = "";
      appSubmitted = "";
      toolChoice = "";
      buttonFollowUp = null;
      toolFilePath = internalStorage;
      if (review) void api.discard(review.id).catch(() => {});
      review = null;
      if (bulkReview) void api.discardBulk(bulkReview.id).catch(() => {});
      bulkReview = null;
      if (device) outcome = null;
    }
  });
  function close() {
    if (review) void api.discard(review.id).catch(() => {});
    review = null;
  }
  function acceptReview(r: Review) {
    if (device && sameTarget(targetOf(device), r.target)) review = r;
    else void api.discard(r.id).catch(() => {});
  }
  function acceptBulk(r: BulkReview) {
    if (device && sameTarget(targetOf(device), r.target)) bulkReview = r;
    else void api.discardBulk(r.id).catch(() => {});
  }
  function closeBulk() {
    if (bulkReview) void api.discardBulk(bulkReview.id).catch(() => {});
    bulkReview = null;
  }
  function result(r: Operation) {
    if (device && !sameTarget(targetOf(device), r.target)) return;
    // A verified file change shows in the list itself, and History keeps the record.
    const quiet = ["file", "setup", "mapping"].includes(r.kind);
    if (!quiet || (r.status !== "verified" && r.status !== "sent")) outcome = r;
    reload++;
  }
</script>

{#if page === "history"}{#key reload}<HistoryPage
      {api}
      onreview={acceptReview}
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
        onbulk={acceptBulk}
        bind:filter={appFilter}
        bind:search={appSearch}
        bind:submitted={appSubmitted}
        onreview={acceptReview}
        oninstall={() => onnavigate("install")}
      />
    {:else if page === "remote"}<RemotePage
        {device}
        {api}
        onresult={(r) => {
          if (r.status !== "sent") outcome = r;
        }}
        onapps={() => onnavigate("apps")}
        onbuttons={() => {
          toolChoice = "buttons";
          onnavigate("tools");
        }}
      />
    {:else if page === "tools"}<ToolsPage
        {device}
        {api}
        bind:tool={toolChoice}
        bind:buttonFollowUp
        bind:filePath={toolFilePath}
        onreview={acceptReview}
        onbulk={acceptBulk}
        oninstall={() => onnavigate("install")}
      />{/if}
  {/key}{/if}
<BulkActionReview
  review={bulkReview}
  {api}
  deviceName={device?.model ?? "Original target"}
  onclose={closeBulk}
  onresult={result}
/>
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
    <p class="target-caption">
      {device && sameTarget(targetOf(device), outcome.target)
        ? device.model
        : `Device ${outcome.target.deviceId.slice(-8)}`} · Profile {outcome
        .target.profileId}
    </p>
    <p class={`outcome ${outcome.status}`}>{outcome.status}</p>
    <Output
      text={outcome.error ||
        resultText(outcome.kind, outcome.output) ||
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
