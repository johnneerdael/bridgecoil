<script lang="ts">
  import RatingBadge from "./RatingBadge.svelte";
  import { levelInfo } from "./ratings";
  import type { Platform, BulkReview, Operation } from "./client";
  import { errorMessage } from "../model";
  import Modal from "../Modal.svelte";
  let {
    review,
    api,
    deviceName,
    onclose,
    onresult,
  }: {
    review: BulkReview | null;
    api: Platform;
    deviceName: string;
    onclose: () => void;
    onresult: (r: Operation) => void;
  } = $props();
  let busy = $state(false),
    confirmed = $state(false),
    stopAfterFailure = $state(true),
    cancelling = $state(false),
    error = $state("");
  $effect(() => {
    review;
    confirmed = false;
    error = "";
    stopAfterFailure = true;
    cancelling = false;
  });
  const items = $derived(
    review
      ? [
          ...review.items.map((i) => ({
            package: i.package,
            text: i.review.effect,
            losses: i.review.losses,
            excluded: false,
            rating: i.review.rating ?? null,
          })),
          ...review.excluded.map((i) => ({
            package: i.package,
            text: i.reason,
            losses: [],
            excluded: true,
            rating: i.rating ?? null,
          })),
        ]
      : [],
  );
  async function apply() {
    if (!review || !confirmed || busy) return;
    busy = true;
    error = "";
    try {
      const r = await api.applyBulk(review.id, stopAfterFailure);
      onresult(r);
      onclose();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function cancel() {
    if (!review) return;
    cancelling = true;
    try {
      await api.cancelBulk(review.id);
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<Modal
  dismissible={!busy}
  open={!!review}
  title="Review app changes"
  onclose={() => {
    if (!busy) onclose();
  }}
  >{#if review}<p class="target-caption">
      {deviceName} · Profile {review.target.profileId} · {review.items.length} included,
      {review.excluded.length} excluded
    </p>
    {#if busy}<p role="status">
        {cancelling
          ? "Cancellation requested. Waiting for the current item."
          : "Applying reviewed items. Completed changes stay applied."}
      </p>
      <button class="secondary" disabled={cancelling} onclick={cancel}
        >Cancel remaining items</button
      >
    {:else}
      {#if review.items[0]}<h3>{review.items[0].review.title}</h3>{/if}
      {#each [...new Set(review.items.flatMap((item) => item.review.losses))] as loss}<p
          class="risk-note"
        >
          {loss}
        </p>{/each}
      <p class="subtle">
        One confirmation applies to the included apps. Open an app below to
        inspect its impact.
      </p>
      <div class="bulk-review-list">
        {#each items as item (item.package)}<details class="bulk-review-item">
            <summary
              ><strong>{item.package}</strong><span class="subtle"
                >{item.excluded ? "Excluded · will not run" : "Included"}</span
              ></summary
            >
            <p>{item.text}</p>
            {#if item.rating === "expert" || item.rating === "unsafe"}<div
                class="risk-note rating-warning"
              >
                <RatingBadge level={item.rating} />
                <p>{levelInfo[item.rating].explanation}</p>
              </div>{:else if item.rating}<RatingBadge
                level={item.rating}
              />{/if}
          </details>{/each}
      </div>
      <label class="checkbox"
        ><input type="checkbox" bind:checked={stopAfterFailure} /><span
          >Stop after the first failed or unknown item</span
        ></label
      ><label class="checkbox"
        ><input type="checkbox" bind:checked={confirmed} /><span
          >Apply the included changes. Completed items stay changed if a later
          item fails.</span
        ></label
      ><button
        class="primary"
        disabled={!confirmed || !review.items.length}
        onclick={apply}
        >Apply {review.items.length}
        {review.items.length === 1 ? "change" : "changes"}</button
      >{/if}
    {#if error}<p role="alert" class="error-text">{error}</p>{/if}{/if}</Modal
>

<style>
  .bulk-review-list {
    max-height: 36dvh;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .bulk-review-item {
    border-bottom: 1px solid var(--line);
    padding: 10px 0;
  }
  summary {
    cursor: pointer;
    min-height: 44px;
  }
  summary strong {
    overflow-wrap: anywhere;
    font-size: 12px;
  }
  summary span {
    display: block;
    padding-top: 4px;
  }
  summary:focus-visible {
    outline: 3px solid var(--bc-focus);
    outline-offset: -3px;
  }
</style>
