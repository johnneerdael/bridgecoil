<script lang="ts">
  import Pager from "./Pager.svelte";
  import { outputPages } from "./output";
  let { text }: { text: string } = $props();
  let page = $state(0);
  const pages = $derived(outputPages(text));
  $effect(() => {
    text;
    page = 0;
  });
</script>

<pre class="output">{pages[page] || "No output returned."}</pre>
{#if pages.length > 1}<Pager
    {page}
    total={pages.length}
    size={1}
    onpage={(n) => (page = n)}
  />{/if}
