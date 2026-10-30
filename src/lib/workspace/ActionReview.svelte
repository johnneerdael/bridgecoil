<script lang="ts">
  import Modal from "../Modal.svelte";
  import type { Platform, Review, Operation } from "./client";
  import { errorMessage } from "../model";
  import Output from "./Output.svelte";
  let {
    review,
    deviceName = "Connected device",
    api,
    onclose,
    onresult,
  }: {
    review: Review | null;
    deviceName?: string;
    api: Platform;
    onclose: () => void;
    onresult: (r: Operation) => void;
  } = $props();
  let busy = $state(false);
  let confirmed = $state(false);
  let error = $state("");
  let tab = $state("effect");
  $effect(() => {
    review;
    confirmed = false;
    error = "";
    tab = "effect";
  });
  async function apply() {
    if (!review || busy || !confirmed) return;
    busy = true;
    error = "";
    try {
      const r = await api.apply(review.id);
      onresult(r);
      onclose();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  open={!!review}
  title="Review change"
  onclose={() => {
    if (!busy) onclose();
  }}
>
  {#if review}
    <h3>{review.title}</h3>
    <p class="target-caption">
      {deviceName} · Profile {review.target.profileId}
    </p>
    <div class="segmented">
      <button class:chosen={tab === "effect"} onclick={() => (tab = "effect")}
        >Effect</button
      ><button
        class:chosen={tab === "commands"}
        onclick={() => (tab = "commands")}>Commands</button
      >
    </div>
    {#if tab === "effect"}<p class="subtle">{review.currentState}</p>
      <p>{review.effect}</p>
      {#each review.losses as loss}<p class="risk-note">{loss}</p>{/each}
      <p class="subtle">
        {review.reversible
          ? "A supported inverse can be reviewed from History after verification."
          : "No automatic undo is available."}
      </p>
    {:else}<Output text={review.commands.join("\n")} />{/if}
    {#if error}<p role="alert" class="error-text">{error}</p>{/if}
    <label class="checkbox"
      ><input type="checkbox" bind:checked={confirmed} disabled={busy} /><span
        >Apply this change to the reviewed target</span
      ></label
    >
    <button class="primary" disabled={busy || !confirmed} onclick={apply}
      >{busy ? "Applying…" : "Apply change"}</button
    >
  {/if}
</Modal>
