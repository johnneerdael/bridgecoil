<script lang="ts">
  import Icon from "./Icon.svelte";
  import { levelInfo, type Level } from "./ratings";
  let { level }: { level: Level | null } = $props();
  const key = $derived(level ?? "notRated");
</script>

<span class={`rating-badge ${key}`}
  ><Icon name={levelInfo[key].icon} size={12} />{levelInfo[key].label}</span
>
