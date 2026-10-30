<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    open,
    title,
    onclose,
    children,
    variant = "dialog",
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    variant?: "dialog" | "drawer";
  } = $props();
  let element = $state<HTMLDialogElement>();
  const titleId = `dialog-${crypto.randomUUID()}`;
  $effect(() => {
    if (!element) return;
    if (open && !element.open) element.showModal();
    else if (!open && element.open) element.close();
  });
  function backdrop(event: MouseEvent) {
    if (event.target !== element || !element) return;
    const r = element.getBoundingClientRect();
    if (
      event.clientX < r.left ||
      event.clientX > r.right ||
      event.clientY < r.top ||
      event.clientY > r.bottom
    )
      onclose();
  }
</script>

<dialog
  bind:this={element}
  class="modal"
  class:drawer={variant === "drawer"}
  aria-labelledby={titleId}
  {onclose}
  onclick={backdrop}
>
  <div class="modal-panel">
    <header class="modal-heading">
      <h2 id={titleId}>{title}</h2>
      <button class="icon-button" onclick={onclose} aria-label="Close dialog"
        >×</button
      >
    </header>
    <div class="modal-body">{@render children()}</div>
  </div>
</dialog>
