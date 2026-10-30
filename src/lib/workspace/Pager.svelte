<script lang="ts">
  let {
    page,
    total,
    size,
    onpage,
    busy = false,
  }: {
    page: number;
    total: number;
    size: number;
    onpage: (n: number) => void;
    busy?: boolean;
  } = $props();
</script>

{#if total > size}<div class="pager">
    <button
      class="secondary"
      disabled={busy || page === 0}
      onclick={() => onpage(page - 1)}>Previous</button
    ><span aria-live="polite"
      >{total ? page + 1 : 0} / {Math.ceil(total / size)}</span
    ><button
      class="secondary"
      disabled={busy || (page + 1) * size >= total}
      onclick={() => onpage(page + 1)}>Next</button
    >
  </div>{/if}
