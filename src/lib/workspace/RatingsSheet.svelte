<script lang="ts">
  import Modal from "../Modal.svelte";
  import { errorMessage } from "../model";
  import type { Platform } from "./client";
  import type { RatingsStatus } from "./ratings";
  let {
    open,
    api,
    status,
    onclose,
    onchange,
  }: {
    open: boolean;
    api: Platform;
    status: RatingsStatus | null;
    onclose: () => void;
    onchange: () => void;
  } = $props();
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  async function run(work: () => Promise<string>) {
    busy = true;
    error = "";
    notice = "";
    try {
      notice = await work();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const update = () =>
    run(async () => {
      const changed = await api.ratingsUpdate();
      if (changed) onchange();
      return changed ? "Ratings updated." : "You already have the latest list.";
    });
  const remove = () =>
    run(async () => {
      await api.ratingsRemove();
      onchange();
      onclose();
      return "";
    });
</script>

<Modal {open} title="Ratings" {onclose}>
  {#if open}<div class="file-upload">
      <p>
        Community removal ratings from the Universal Android Debloater list.
        Community list, mostly based on phones.
      </p>
      <p class="subtle break-word">
        github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation
      </p>
      {#if status?.stored}<p class="subtle">
          {status.packages} packages · commit {status.commit?.slice(0, 7)} ·
          {status.committedAt
            ? new Date(status.committedAt).toLocaleDateString()
            : "date unknown"}
        </p>
        <button class="secondary" disabled={busy} onclick={update}
          >Check for update</button
        ><button class="secondary" disabled={busy} onclick={remove}
          >Remove ratings</button
        >
      {:else}<button class="primary" disabled={busy} onclick={update}
          >Download ratings</button
        >{/if}
      {#if error}<p class="error-text" role="alert">{error}</p>{/if}
      {#if notice}<p class="subtle" role="status">{notice}</p>{/if}
    </div>{/if}
</Modal>
