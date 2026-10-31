<script lang="ts">
  import RatingBadge from "./RatingBadge.svelte";
  import { levelInfo } from "./ratings";
  import type { Platform, BulkReview, Operation } from "./client";
  import { errorMessage } from "../model";
  import Modal from "../Modal.svelte";
  import Pager from "./Pager.svelte";
  import Output from "./Output.svelte";
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
  let page = $state(0),
    busy = $state(false),
    confirmed = $state(false),
    stopAfterFailure = $state(true),
    cancelling = $state(false),
    error = $state("");
  let detail = $state(false);
  $effect(() => {
    review;
    page = 0;
    confirmed = false;
    error = "";
    stopAfterFailure = true;
    cancelling = false;
    detail = false;
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
    {:else}{@const item = items[page]}{#if item}<h3 class="break-word">
          {item.package}
        </h3>
        <p class="subtle">
          {item.excluded ? "Excluded · will not run" : "Included"}
        </p>
        {#if item.rating === "expert" || item.rating === "unsafe"}<div
            class="risk-note rating-warning"
          >
            <RatingBadge level={item.rating} />
            <p>{levelInfo[item.rating].explanation}</p>
          </div>{:else if item.rating}<RatingBadge level={item.rating} />{/if}
        <button class="text-action" onclick={() => (detail = !detail)}
          >{detail ? "Show review" : "Inspect this item's impact"}</button
        >{#if detail}<Output
            text={[item.text, ...item.losses].join("\n\n")}
          />{:else}<p>{item.text}</p>
          {#if item.losses.length}<p class="risk-note">
              This action can delete app data. Inspect its impact before
              applying.
            </p>{/if}{/if}{/if}<Pager
        {page}
        total={items.length}
        size={1}
        onpage={(n) => {
          page = n;
          detail = false;
        }}
      />
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
