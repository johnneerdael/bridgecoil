<script lang="ts">
  import DevicePage from "../../src/lib/workspace/DevicePage.svelte";
  import AppsPage from "../../src/lib/workspace/AppsPage.svelte";
  import RemotePage from "../../src/lib/workspace/RemotePage.svelte";
  import ToolsPage from "../../src/lib/workspace/ToolsPage.svelte";
  import HistoryPage from "../../src/lib/workspace/HistoryPage.svelte";
  import ActionReview from "../../src/lib/workspace/ActionReview.svelte";
  import BulkReviewView from "../../src/lib/workspace/BulkReview.svelte";
  import type { Device } from "../../src/lib/model";
  import type {
    Destination,
    Review,
    BulkReview,
  } from "../../src/lib/workspace/client";
  import { device as fixtureDevice, api } from "./platform";
  let device = $state<Device>(fixtureDevice),
    page = $state<Destination>("device"),
    review = $state<Review | null>(null),
    bulk = $state<BulkReview | null>(null),
    keyboard = $state(false),
    dark = $state(false);
  const titles = {
    device: "Device",
    apps: "Apps",
    remote: "Remote",
    tools: "Tools",
    history: "History",
    connect: "Connection fixture",
    install: "Install fixture",
  };
  function navigate(next: Destination) {
    page = next === "connect" ? "device" : next === "install" ? "apps" : next;
  }
  function focus(event: FocusEvent) {
    keyboard =
      event.target instanceof HTMLTextAreaElement ||
      (event.target instanceof HTMLInputElement &&
        event.target.type !== "checkbox");
  }
</script>

<div
  class="workspace"
  class:keyboard
  onfocusin={focus}
  onfocusout={() => (keyboard = false)}
>
  <header class="fixture-header">
    <strong>UI fixture · no device connection</strong>
    <label class="sr-only" for="fixture-screen">Fixture screen</label><select id="fixture-screen" bind:value={page}>{#each ["device","apps","remote","tools","history"] as id}<option value={id}>{titles[id as Destination]}</option>{/each}</select>
    <button class="secondary" onclick={()=>{dark=!dark;document.documentElement.dataset.theme=dark?"dark":"light";}}>Theme</button>
  </header>
  <main>
    <header class="page-heading"><h1>{titles[page]}</h1></header>
    <div class="page-content">
      {#if page === "device"}<DevicePage
          {device}
          {api}
          ondevice={(d) => (device = d)}
          onnavigate={navigate}
        />
      {:else if page === "apps"}<AppsPage
          {device}
          {api}
          onreview={(r) => (review = r)}
          onbulk={(r) => (bulk = r)}
          oninstall={() => {}}
        />
      {:else if page === "remote"}<RemotePage
          {device}
          {api}
          onresult={() => {}}
          onapps={() => (page = "apps")}
        />
      {:else if page === "tools"}<ToolsPage
          {device}
          {api}
          onreview={(r) => (review = r)}
          oninstall={() => (page = "apps")}
        />
      {:else if page === "history"}<HistoryPage
          {api}
          onreview={(r) => (review = r)}
        />{/if}
    </div>
  </main>
</div>
<ActionReview
  {review}
  {api}
  deviceName={device.model}
  onclose={() => (review = null)}
  onresult={() => {}}
/>
<BulkReviewView
  review={bulk}
  {api}
  deviceName={device.model}
  onclose={() => (bulk = null)}
  onresult={() => {}}
/>

<style>
.fixture-header{display:grid;grid-template-columns:minmax(0,1fr) 100px 60px;align-items:center;gap:8px;padding:6px 12px;min-height:56px;border-bottom:1px solid var(--line);font-size:10px;color:var(--bc-warning);background:var(--bc-warningSoft);}
.fixture-header button{padding:8px;font-size:11px;min-height:44px;}
</style>
