<script lang="ts">
  let { name, size = 22 }: { name: string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  ><use href={`/brand/icons/ui/bridgecoil-sprite.svg#bc-${name}`} /></svg
>
