<script lang="ts">
  import type { Platform, Target, Review } from "./client";
  import { errorMessage } from "../model";
  import Pager from "./Pager.svelte";
  let {
    api,
    target,
    packageId,
    onreview,
  }: {
    api: Platform;
    target: Target;
    packageId: string;
    onreview: (review: Review) => void;
  } = $props();
  type Policy = {
    op: string;
    name: string;
    mode: { packageMode: string; uidMode: string | null } | null;
    error: string | null;
  };
  let policies = $state<Policy[]>([]),
    page = $state(0),
    busy = $state(false),
    error = $state("");
  $effect(() => {
    const current = target,
      id = packageId;
    let cancelled = false;
    busy = true;
    error = "";
    policies = [];
    void api
      .query<Policy[]>(current, { kind: "appOps", package: id })
      .then((rows) => {
        if (!cancelled) policies = rows;
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
  async function change(op: string, mode: string) {
    busy = true;
    error = "";
    try {
      onreview(
        await api.prepare(target, {
          kind: "appOp",
          package: packageId,
          op,
          mode,
        }),
      );
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<p class="subtle">
  Review a stored Android policy. Other permissions and system rules can still
  limit effective access.
</p>
{#if busy}<p class="subtle" role="status">Inspecting policies…</p>{/if}
{#each policies.slice(page * 2, page * 2 + 2) as policy}<div
    class="setting-row"
  >
    <label for={`op-${policy.op}`}
      >{policy.name}<small
        >{policy.mode
          ? `Stored: ${policy.mode.packageMode}${policy.mode.uidMode ? ` · UID override: ${policy.mode.uidMode}` : ""}`
          : "Unavailable on this TV"}</small
      ></label
    ><select
      id={`op-${policy.op}`}
      disabled={busy || !policy.mode}
      value={policy.mode?.packageMode ?? ""}
      onchange={(event) => change(policy.op, event.currentTarget.value)}
      >{#if !policy.mode}<option value="">Unknown</option
        >{/if}{#each [["default", "Android default"], ["allow", "Allow"], ["ignore", "Ignore requests"], ["deny", "Deny"], ["foreground", "Foreground only"]] as [value, label]}<option
          {value}>{label}</option
        >{/each}</select
    >
  </div>{/each}
{#if error}<p role="alert" class="error-text">{error}</p>{/if}<Pager
  {page}
  total={policies.length}
  size={2}
  {busy}
  onpage={(n) => (page = n)}
/>
