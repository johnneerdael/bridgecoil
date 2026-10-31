<script lang="ts">
  import RatingBadge from "./RatingBadge.svelte";
  import { levelInfo, type Rating } from "./ratings";
  let {
    rating,
    installedRelated,
    listDate,
  }: {
    rating: Rating | null;
    installedRelated: string[];
    listDate: string | null;
  } = $props();
  const level = $derived(rating?.level ?? null);
</script>

{#snippet related(title: string, packages: string[])}
  <div>
    <dt>{title}</dt>
    {#if packages.length}{#each packages as name}<dd class="break-word">
          {name}{#if installedRelated.includes(name)}&nbsp;· On this TV{/if}
        </dd>{/each}{:else}<dd>None listed</dd>{/if}
  </div>
{/snippet}

<section class="rating-detail">
  <RatingBadge {level} />
  {#if rating}<p class="subtle">{levelInfo[level ?? "notRated"].explanation}</p>
    {#if rating.description}<p class="rating-description">
        {rating.description}
      </p>{/if}
    <dl>
      {#if rating.source}<div>
          <dt>Source</dt>
          <dd>{rating.source}</dd>
        </div>{/if}
      {@render related("Needs", rating.dependencies)}
      {@render related("Needed by", rating.neededBy)}
    </dl>
    {#if !rating.dependencies.length || !rating.neededBy.length}<p
        class="subtle"
      >
        An empty list here doesn't prove there are none.
      </p>{/if}
  {:else}<p class="subtle">
      UAD has no entry for this package. That means unknown, not low risk.
    </p>{/if}
  <p class="subtle">
    Community rating, mostly based on phones{listDate
      ? ` · list from ${new Date(listDate).toLocaleDateString()}`
      : ""}.
  </p>
</section>
