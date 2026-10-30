<script lang="ts">
  import { errorMessage } from "../model";
  import type { Platform, Operation, Page, Review } from "./client";
  import Pager from "./Pager.svelte";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  let { api, onreview }: { api: Platform; onreview: (r: Review) => void } =
    $props();
  let page = $state(0);
  let reload = $state(0);
  let busy = $state(false);
  let error = $state("");
  let result = $state<Page<Operation>>({
    items: [],
    total: 0,
    page: 0,
    pageSize: 4,
  });
  let detail = $state<Operation | null>(null);
  let clearing = $state(false);
  let tab = $state("summary");
  $effect(() => {
    const p = page;
    reload;
    let cancelled = false;
    busy = true;
    error = "";
    void api
      .history(p, 4)
      .then((r) => {
        if (!cancelled) result = r;
      })
      .catch((e) => {
        if (!cancelled) error = errorMessage(e);
      })
      .finally(() => {
        if (!cancelled) busy = false;
      });
    return () => {
      cancelled = true;
    };
  });
  async function recover(id: string) {
    busy = true;
    error = "";
    try {
      const r = await api.recovery(id);
      detail = null;
      onreview(r);
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function clear() {
    busy = true;
    error = "";
    try {
      await api.clearHistory();
      clearing = false;
      detail = null;
      page = 0;
      reload++;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="feature-page">
  <p class="subtle">Local records · interrupted changes are never replayed.</p>
  {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  <div class="row-list">
    {#each result.items as row}<button
        class="list-row"
        onclick={() => {
          detail = row;
          tab = "summary";
        }}
        ><span class="truncate"
          ><strong>{row.title}</strong><small
            >{new Date(row.createdMs).toLocaleString()} · Profile {row.target
              .profileId}</small
          ></span
        ><span class={`outcome ${row.status}`}>{row.status}</span></button
      >{/each}{#if !busy && !result.items.length}<p class="empty-caption">
        No operations recorded yet.
      </p>{/if}
  </div>
  <Pager
    {page}
    total={result.total}
    size={4}
    {busy}
    onpage={(n) => (page = n)}
  />
  <div class="page-footer">
    <button class="secondary" disabled={busy} onclick={() => reload++}
      >Refresh</button
    ><button
      class="secondary"
      disabled={busy || !result.total}
      onclick={() => (clearing = true)}>Clear records</button
    >
  </div>
</section>
<Modal open={!!detail} title="Operation details" onclose={() => (detail = null)}
  >{#if detail}<h3 class="break-word">{detail.title}</h3>
    <div class="segmented">
      <button class:chosen={tab === "summary"} onclick={() => (tab = "summary")}
        >Summary</button
      ><button class:chosen={tab === "output"} onclick={() => (tab = "output")}
        >Output</button
      >
    </div>
    {#if tab === "summary"}<dl>
        <div>
          <dt>Result</dt>
          <dd>{detail.status}</dd>
        </div>
        <div>
          <dt>Profile</dt>
          <dd>{detail.target.profileId}</dd>
        </div>
        <div>
          <dt>Time</dt>
          <dd>{new Date(detail.createdMs).toLocaleString()}</dd>
        </div>
      </dl>
      <p class="subtle">
        {detail.status === "sent"
          ? "The command returned successfully. Its physical effect was not verified."
          : detail.status === "unknown"
            ? "Inspect the TV before considering another attempt."
            : detail.status === "verified"
              ? "Expected state was read back from the target."
              : "Read the output for the recorded result."}
      </p>
      {#if detail.error}<p class="risk-note">{detail.error}</p>{/if}
      {#if detail.recovery}<button
          class="secondary"
          disabled={busy}
          onclick={() => detail && recover(detail.id)}>Review recovery</button
        >{/if}
    {:else}<Output text={detail.output} />{/if}{#if error}<p
        class="error-text"
        role="alert"
      >
        {error}
      </p>{/if}{/if}</Modal
>
<Modal
  open={clearing}
  title="Clear local history?"
  onclose={() => (clearing = false)}
  ><p>
    This removes Bridgecoil's local operation records and recorded recovery
    options. It makes no changes to the TV.
  </p>
  <button class="primary" disabled={busy} onclick={clear}
    >Clear local records</button
  ></Modal
>
