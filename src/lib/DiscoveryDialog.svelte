<script lang="ts">
  import Modal from "./Modal.svelte";
  import { errorMessage } from "./model";
  import {
    serviceLabel,
    displayServices,
    type DiscoveryResult,
    type DiscoveredService,
  } from "./discovery";
  let {
    open,
    discover,
    onselect,
    onclose,
    connected = false,
  }: {
    open: boolean;
    discover: () => Promise<DiscoveryResult>;
    onselect: (service: DiscoveredService) => void;
    onclose: () => void;
    connected?: boolean;
  } = $props();
  let loading = $state(false),
    error = $state(""),
    refresh = $state(0);
  let result = $state<DiscoveryResult>({ services: [], warnings: [] });
  $effect(() => {
    if (!open) return;
    refresh;
    let active = true;
    loading = true;
    error = "";
    result = { services: [], warnings: [] };
    void discover()
      .then((value) => {
        if (active) result = value;
      })
      .catch((e) => {
        if (active) error = errorMessage(e);
      })
      .finally(() => {
        if (active) loading = false;
      });
    return () => {
      active = false;
    };
  });
</script>

<Modal {open} title="Nearby ADB devices" {onclose}>
  <div class="discovery-content">
    {#if loading}<p class="subtle" role="status">
        Searching your local network…
      </p>
    {:else if error}<p class="error-text" role="alert">{error}</p>
    {:else if !result.services.length}<p class="subtle" role="status">
        No ADB services found. Check that the TV and phone are on the same
        network and Wireless debugging is on. A TV on a network cable usually
        can't use it, so try Wi-Fi.
      </p>{/if}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="discovery-results"
      role="region"
      aria-label="Discovered ADB services"
      tabindex="0"
    >
      {#each displayServices(result.services) as service}
        <button class="list-row" onclick={() => onselect(service)}>
          <span class="truncate"
            ><strong>{service.name}</strong><small
              >{serviceLabel[service.kind]} · {service.endpoint}</small
            ></span
          ><span aria-hidden="true">→</span>
        </button>
      {/each}
    </div>
    {#if result.warnings.length}<p class="subtle" role="status">
        {result.warnings.join(" ")}
      </p>{/if}
    <p class="subtle discovery-note">
      Open the TV’s pairing-code screen to find its pairing port. Choosing a
      result fills its address.{connected
        ? " Choosing another TV disconnects the current one."
        : ""}
    </p>
    <button class="secondary" disabled={loading} onclick={() => refresh++}
      >{loading ? "Searching…" : "Find again"}</button
    >
  </div>
</Modal>

<style>
  :global(.modal-body:has(.discovery-content)) {
    display: flex;
    overflow: hidden;
  }
  .discovery-content {
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 100%;
    height: min(440px, calc(100dvh - 140px));
    gap: 8px;
  }
  .discovery-content > * {
    flex-shrink: 0;
  }
  .discovery-results {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    overscroll-behavior: contain;
    flex: 1 1 auto;
    min-height: 44px;
    padding: 2px;
  }
  .discovery-results:empty {
    display: none;
  }
  .discovery-results > button {
    flex-shrink: 0;
  }
  .discovery-note {
    margin: 0;
    font-size: 11px;
    line-height: 1.4;
  }
</style>
