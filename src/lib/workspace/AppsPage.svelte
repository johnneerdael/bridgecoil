<script lang="ts">
  import { onMount } from "svelte";
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Platform,
    type Review,
    type Action,
    type PackageAction,
    type Page,
    type PackageDetail,
    type AppMetadata,
    type BulkReview,
  } from "./client";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  import Pager from "./Pager.svelte";
  import Icon from "./Icon.svelte";
  import AppOps from "./AppOps.svelte";
  import RatingBadge from "./RatingBadge.svelte";
  import RatingDetail from "./RatingDetail.svelte";
  import RatingsSheet from "./RatingsSheet.svelte";
  import {
    groupLabels,
    levelInfo,
    type Group,
    type RatingsStatus,
  } from "./ratings";
  let {
    device,
    api,
    onreview,
    onbulk,
    oninstall,
    filter = $bindable("all"),
    search = $bindable(""),
    submitted = $bindable(""),
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
    onbulk: (r: BulkReview) => void;
    oninstall: () => void;
    filter?: string;
    search?: string;
    submitted?: string;
  } = $props();
  const pageSize = 32;
  let list = $state<HTMLDivElement>();
  let loadingMore = $state(false);
  let loadError = $state("");
  let nextPage = 1;
  let generation = 0;
  let selecting = $state(false),
    selected = $state<string[]>([]),
    bulkAction = $state<PackageAction>("disable");
  function toggle(id: string) {
    selected = selected.includes(id)
      ? selected.filter((p) => p !== id)
      : selected.length < 32
        ? [...selected, id]
        : selected;
  }
  async function reviewBulk() {
    if (!selected.length || busy) return;
    busy = true;
    error = "";
    try {
      onbulk(await api.prepareBulk(targetOf(device), selected, bulkAction));
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  let refresh = $state(0);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  let activitiesOpen = $state(false),
    activityPage = $state(0);
  type AppPage = Page<AppMetadata> & {
    metadataNote?: string | null;
    groups?: { id: Group; count: number }[];
  };
  let result = $state<AppPage>({
    items: [],
    total: 0,
    page: 0,
    pageSize,
  });
  const cardDismissedKey = "bridgecoil.ratingsCardDismissed";
  function storedDismissal() {
    try {
      return localStorage.getItem(cardDismissedKey) !== null;
    } catch {
      return false;
    }
  }
  let ratings = $state<RatingsStatus | null>(null);
  let cardDismissed = $state(storedDismissal());
  let ratingsOpen = $state(false);
  let ratingsBusy = $state(false);
  let ratingsError = $state("");
  const showRatingsCard = $derived(
    ratings !== null && !ratings.stored && !cardDismissed,
  );
  async function loadRatings() {
    try {
      ratings = await api.ratingsStatus();
    } catch {
      ratings = null;
    }
  }
  function ratingsChanged() {
    void loadRatings();
    refresh++;
  }
  async function downloadRatings() {
    ratingsBusy = true;
    ratingsError = "";
    try {
      await api.ratingsUpdate();
      ratingsChanged();
    } catch (e) {
      ratingsError = errorMessage(e);
    } finally {
      ratingsBusy = false;
    }
  }
  function dismissRatingsCard() {
    cardDismissed = true;
    try {
      localStorage.setItem(cardDismissedKey, "1");
    } catch {
      // Only a convenience: the card returns next time.
    }
  }
  onMount(() => {
    void loadRatings();
    // Failures stay silent here; the Ratings sheet reports them when asked. A new list
    // applies on the next load, so it can't clear a selection or close an open app.
    void (async () => {
      try {
        if (await api.ratingsUpdateIfDue()) await loadRatings();
      } catch {
        return;
      }
    })();
  });
  let unsafeOpen = $state(false);
  const groupCount = (id: Group) =>
    result.groups?.find((group) => group.id === id)?.count ?? 0;
  const unsafeCollapsed = $derived(
    groupCount("unsafe") > 0 && !unsafeOpen && !submitted,
  );
  // Unsafe is always last, so the list simply stops loading where it would begin.
  const atCollapsedUnsafe = $derived(
    unsafeCollapsed &&
      result.items.length >= result.total - groupCount("unsafe"),
  );
  const showsBadge = (app: AppMetadata) =>
    !!app.group && (app.group !== "installed" || !!app.rating?.level);
  let detail = $state<PackageDetail | null>(null);
  let tab = $state("overview");
  let actionPage = $state(0);
  let permissionPage = $state(0);
  let accessTab = $state("runtime");
  const runtimePermissions = $derived(
    detail?.metadata?.permissions?.filter((p) => p.protection === 1) ?? [],
  );
  const permissionLabel = (name: string) =>
    name.split(".").pop()!.toLowerCase().replaceAll("_", " ");
  const actions: { id: PackageAction; label: string }[] = [
    { id: "enable", label: "Enable" },
    { id: "disable", label: "Disable" },
    { id: "forceStop", label: "Force stop" },
    { id: "remove", label: "Remove from profile" },
    { id: "restore", label: "Restore existing package" },
    { id: "clearCache", label: "Clear cache" },
    { id: "clearData", label: "Clear app data" },
    { id: "resetEnabled", label: "Default enabled state" },
  ];
  const blockedOnUnsafe: PackageAction[] = ["disable", "remove", "clearData"];
  let unsafeUnlocked = $state(false);
  const unsafeLocked = $derived(
    detail?.rating?.level === "unsafe" && !unsafeUnlocked,
  );
  const detailActions = $derived(
    unsafeLocked
      ? actions.filter((a) => !blockedOnUnsafe.includes(a.id))
      : actions,
  );
  $effect(() => {
    const target = targetOf(device);
    const q = {
      kind: "packages" as const,
      filter,
      search: submitted,
      page: 0,
      pageSize,
    };
    refresh;
    generation++;
    let cancelled = false;
    busy = true;
    loadingMore = false;
    loadError = "";
    nextPage = 1;
    selected = [];
    if (list) list.scrollTop = 0;
    error = "";
    result = { items: [], total: 0, page: 0, pageSize };
    detail = null;
    void api
      .query<AppPage>(target, q)
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
  async function loadMore() {
    if (
      busy ||
      loadingMore ||
      loadError ||
      atCollapsedUnsafe ||
      result.items.length >= result.total
    )
      return;
    const request = generation;
    loadingMore = true;
    try {
      const more = await api.query<AppPage>(targetOf(device), {
        kind: "packages",
        filter,
        search: submitted,
        page: nextPage,
        pageSize,
      });
      if (request !== generation) return;
      const seen = new Set(result.items.map((app) => app.package));
      const additional = more.items.filter((app) => !seen.has(app.package));
      result = {
        ...result,
        items: [...result.items, ...additional],
        total: additional.length ? more.total : result.items.length,
        metadataNote: result.metadataNote || more.metadataNote,
        groups: more.groups,
      };
      nextPage++;
    } catch (e) {
      if (request === generation) loadError = errorMessage(e);
    } finally {
      if (request === generation) loadingMore = false;
    }
  }
  async function inspect(packageId: string) {
    busy = true;
    error = "";
    notice = "";
    activityPage = 0;
    activitiesOpen = false;
    unsafeUnlocked = false;
    const id = device.id;
    try {
      const r = await api.query<PackageDetail>(targetOf(device), {
        kind: "package",
        package: packageId,
      });
      if (device.id === id) {
        detail = r;
        tab = "overview";
        actionPage = 0;
        permissionPage = 0;
        accessTab = "runtime";
      }
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function plan(action: Action, allowUnsafe = false) {
    busy = true;
    error = "";
    try {
      const r = allowUnsafe
        ? await api.prepare(targetOf(device), action, true)
        : await api.prepare(targetOf(device), action);
      detail = null;
      onreview(r);
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  async function launch(component: string) {
    busy = true;
    error = "";
    try {
      const r = await api.remote(targetOf(device), {
        kind: "launch",
        component,
      });
      if (r.status === "sent") notice = "Open request sent. Check the TV.";
      else error = r.error || r.output || "The open request was not verified.";
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

{#snippet appSummary(app: AppMetadata)}
  {#if app.icon}<img
      class="app-icon"
      src={`data:image/png;base64,${app.icon}`}
      alt=""
    />{:else}<Icon name="apps" />{/if}<span class="truncate"
    ><strong>{app.label || app.package}</strong>{#if app.label}<small
        >{app.package}</small
      >{/if}</span
  >{#if showsBadge(app)}<RatingBadge level={app.rating?.level ?? null} />{/if}
{/snippet}
{#snippet groupHeader(group: Group)}
  <h3 class="app-group">
    {#if group === "unsafe"}<button
        aria-expanded={!unsafeCollapsed}
        disabled={!!submitted}
        onclick={() => {
          unsafeOpen = !unsafeOpen;
          void loadMore();
        }}
        ><span>{groupLabels[group]}</span> <span>{groupCount(group)}</span
        ><Icon name={unsafeCollapsed ? "chevron-right" : "chevron-down"} size={14} /></button
      >
    {:else}<span>{groupLabels[group]}</span>
      <span>{groupCount(group)}</span>{/if}
  </h3>
{/snippet}
<section class="feature-page">
  <div class="toolbar">
    <label class="sr-only" for="app-filter">App filter</label><select
      id="app-filter"
      bind:value={filter}
      disabled={busy}
      >{#each [["all", "All apps"], ["user", "User apps"], ["system", "System apps"], ["disabled", "Disabled"], ["removed", "Removed for profile"]] as [value, label]}<option
          {value}>{label}</option
        >{/each}</select
    ><button class="secondary" onclick={oninstall}
      ><Icon name="install" />Install APK</button
    >
  </div>
  <form
    class="search-form"
    onsubmit={(e) => {
      e.preventDefault();
      submitted = search;
      refresh++;
    }}
  >
    <label class="sr-only" for="app-search">Search apps</label><input
      id="app-search"
      bind:value={search}
      placeholder="Find an app"
    /><button class="secondary" disabled={busy} type="submit">Search</button
    ><button
      class="secondary"
      type="button"
      disabled={busy}
      onclick={() => {
        selecting = !selecting;
        if (!selecting) selected = [];
      }}>{selecting ? "Done" : "Select"}</button
    >
  </form>
  {#if selecting}<div class="toolbar">
      <label class="sr-only" for="bulk-action">Bulk app action</label><select
        id="bulk-action"
        bind:value={bulkAction}
        >{#each actions as action}<option value={action.id}
            >{action.label}</option
          >{/each}</select
      ><button
        class="secondary"
        disabled={busy || !selected.length}
        onclick={reviewBulk}
        >Review {selected.length}
        {selected.length === 1 ? "app" : "apps"}</button
      >
    </div>
  {:else}<div class="apps-summary">
      <p class="subtle">
        Profile {device.profileId} · {busy
          ? "Reading apps…"
          : `${result.total} apps`}{result.metadataNote
          ? " · Some app details unavailable"
          : ""}
      </p>
      <button class="text-action" onclick={() => (ratingsOpen = true)}
        >Ratings</button
      >
    </div>{/if}
  {#if showRatingsCard}<aside class="ratings-card">
      <strong>Get community ratings</strong>
      <p class="subtle">
        Downloads the Universal Android Debloater list from GitHub to show what
        each app does and how risky it is to remove.
      </p>
      {#if ratingsError}<p class="error-text" role="alert">
          {ratingsError}
        </p>{/if}
      <div>
        <button
          class="secondary"
          disabled={ratingsBusy}
          onclick={dismissRatingsCard}>Not now</button
        ><button
          class="primary"
          disabled={ratingsBusy}
          onclick={downloadRatings}
          >{ratingsError ? "Try again" : "Download"}</button
        >
      </div>
    </aside>{/if}
  {#if error}<p class="error-text" role="status">{error}</p>{/if}
  <!-- Keyboard users need to focus the scrolling region to use arrow/Page keys. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="row-list app-list"
    bind:this={list}
    role="region"
    aria-label="Installed apps"
    tabindex="0"
    onscroll={(event) => {
      const element = event.currentTarget;
      if (element.scrollHeight - element.scrollTop - element.clientHeight < 240)
        void loadMore();
    }}
  >
    {#each result.items as app, index (app.package)}
      {#if app.group && app.group !== result.items[index - 1]?.group}{@render groupHeader(
          app.group,
        )}{/if}
      {#if app.group === "unsafe" && unsafeCollapsed}<!-- Hidden until expanded. -->
      {:else if selecting}<label class="checkbox list-row bulk-choice"
          ><input
            type="checkbox"
            checked={selected.includes(app.package)}
            disabled={busy ||
              (!selected.includes(app.package) && selected.length >= 32)}
            aria-label={`Select ${app.label || app.package}`}
            onchange={() => toggle(app.package)}
          />{@render appSummary(app)}</label
        >
      {:else}<button
          class="list-row"
          disabled={busy}
          onclick={() => inspect(app.package)}
          >{@render appSummary(app)}<Icon name="chevron-right" /></button
        >{/if}
    {/each}
    {#if atCollapsedUnsafe && result.items.at(-1)?.group !== "unsafe"}{@render groupHeader(
        "unsafe",
      )}{/if}
    {#if !busy && !result.items.length}<p class="empty-caption">
        No apps match this view.
      </p>{/if}
    {#if loadingMore}<p class="subtle" role="status">Reading more apps…</p>{/if}
    {#if loadError}<p class="error-text" role="status">{loadError}</p>
      <button
        class="secondary"
        onclick={() => {
          loadError = "";
          void loadMore();
        }}>Retry loading apps</button
      >{/if}
  </div>
</section>
<RatingsSheet
  open={ratingsOpen}
  {api}
  status={ratings}
  onclose={() => (ratingsOpen = false)}
  onchange={ratingsChanged}
/>
<Modal open={!!detail} title="Application" onclose={() => (detail = null)}>
  {#if detail}<div class="app-detail-heading">
      <h3 class="break-word">
        {detail.metadata?.label || detail.package}
      </h3>
      {#if detail.rating !== undefined}<RatingBadge
          level={detail.rating?.level ?? null}
        />{/if}
    </div>
    {#if detail.metadata?.label}<p class="subtle break-word">
        {detail.package}
      </p>{/if}
    <div class="segmented">
      {#each [["overview", "Overview"], ["actions", "Actions"], ["permissions", "Access"], ["raw", "Details"]] as [section, title]}<button
          class:chosen={tab === section}
          onclick={() => (tab = section)}>{title}</button
        >{/each}
    </div>
    {#if tab === "overview"}{#if detail.rating !== undefined}<RatingDetail
          rating={detail.rating}
          installedRelated={detail.installedRelated ?? []}
          listDate={ratings?.committedAt ?? null}
        />{/if}<dl>
        <div>
          <dt>Version</dt>
          <dd>
            {detail.versionName || "Not reported"} ({detail.state.version ??
              "?"})
          </dd>
        </div>
        <div>
          <dt>Installed for profile</dt>
          <dd>
            {detail.state.installed === null
              ? "Unknown"
              : detail.state.installed
                ? "Yes"
                : "No"}
          </dd>
        </div>
        <div>
          <dt>Enabled state</dt>
          <dd>
            {detail.state.enabled === 0
              ? "Default"
              : detail.state.enabled === 1
                ? "Enabled"
                : detail.state.enabled === 3
                  ? "Disabled for profile"
                  : `State ${detail.state.enabled ?? "unknown"}`}
          </dd>
        </div>
        <div>
          <dt>Stopped</dt>
          <dd>
            {detail.state.stopped === null
              ? "Unknown"
              : detail.state.stopped
                ? "Yes"
                : "No"}
          </dd>
        </div>
      </dl>
      {#if detail.launchActivity || detail.activities.length}<button
          class="primary"
          disabled={busy}
          onclick={() =>
            detail && launch(detail.launchActivity || detail.activities[0])}
          >Open app</button
        >{:else}<p class="subtle">No launch activity reported.</p>{/if}
    {:else if tab === "actions"}{#if unsafeLocked}<div
          class="risk-note rating-warning"
        >
          <p>
            {levelInfo.unsafe.explanation} Disabling, removing and clearing data are
            hidden for this app.
          </p>
          <button
            class="secondary"
            onclick={() => {
              unsafeUnlocked = true;
              actionPage = 0;
            }}>Show these actions anyway</button
          >
        </div>{/if}<div class="action-grid">
        {#each detailActions.slice(actionPage * 4, actionPage * 4 + 4) as action}<button
            class="secondary"
            disabled={busy}
            onclick={() =>
              detail &&
              plan(
                {
                  kind: "package",
                  package: detail.package,
                  action: action.id,
                },
                unsafeUnlocked &&
                  detail.rating?.level === "unsafe" &&
                  blockedOnUnsafe.includes(action.id),
              )}>{action.label}</button
          >{/each}
      </div>
      <Pager
        page={actionPage}
        total={detailActions.length}
        size={4}
        {busy}
        onpage={(n) => (actionPage = n)}
      />
    {:else if tab === "permissions"}
      <div class="segmented">
        {#each [["runtime", "Runtime"], ["special", "Special access"], ["all", "All permissions"]] as [id, label]}<button
            class:chosen={accessTab === id}
            onclick={() => (accessTab = id)}>{label}</button
          >{/each}
      </div>
      {#if accessTab === "special"}<AppOps
          {api}
          target={targetOf(device)}
          packageId={detail.package}
          onreview={(review) => {
            detail = null;
            onreview(review);
          }}
        />
      {:else if accessTab === "all"}<Output
          text={detail.metadata?.permissions
            ?.map(
              (p) =>
                `${p.granted ? "Granted" : "Not granted"} · ${p.name}${p.protection === 1 ? " · runtime" : ""}`,
            )
            .join("\n") ||
            detail.permissions.join("\n") ||
            "No permission details reported."}
        />
      {:else}<div class="permission-list">
          {#each runtimePermissions.slice(permissionPage * 2, permissionPage * 2 + 2) as permission}<div
              class="list-row"
            >
              <span class="truncate"
                ><strong class="capitalize"
                  >{permissionLabel(permission.name)}</strong
                ><small>{permission.granted ? "Granted" : "Not granted"}</small
                ></span
              ><button
                class="secondary"
                disabled={busy}
                aria-label={`Review ${permission.granted ? "revoke" : "grant"} ${permissionLabel(permission.name)}`}
                onclick={() =>
                  detail &&
                  plan({
                    kind: "permission",
                    package: detail.package,
                    permission: permission.name,
                    grant: !permission.granted,
                  })}>{permission.granted ? "Revoke…" : "Grant…"}</button
              >
            </div>{/each}
        </div>
        {#if !runtimePermissions.length}<p class="subtle">
            {detail.metadata
              ? "No changeable runtime permissions reported."
              : "Runtime access could not be inspected. Read-only permission details are available under All permissions."}
          </p>{/if}<Pager
          page={permissionPage}
          total={runtimePermissions.length}
          size={2}
          {busy}
          onpage={(n) => (permissionPage = n)}
        />{/if}
    {:else}{#if detail.activities.length > 1}<button
          class="secondary"
          onclick={() => (activitiesOpen = true)}
          >Choose a launch activity</button
        >{/if}<Output
        text={`Signing certificate SHA-256:\n${detail.metadata?.signers?.join("\n") || "Not reported"}\n\n${detail.raw}`}
      />{/if}
    {#if notice}<p class="subtle" role="status">{notice}</p>{/if}
    {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  {/if}
</Modal>
<Modal
  open={activitiesOpen && !!detail}
  title="Launch activity"
  onclose={() => (activitiesOpen = false)}
>
  <p class="subtle">
    Advanced: choose a launch target reported by the installed app.
  </p>
  {#each detail?.activities.slice(activityPage * 3, activityPage * 3 + 3) ?? [] as component}<button
      class="list-row break-word"
      disabled={busy}
      onclick={() => {
        activitiesOpen = false;
        void launch(component);
      }}>{component}</button
    >{/each}
  <Pager
    page={activityPage}
    total={detail?.activities.length ?? 0}
    size={3}
    {busy}
    onpage={(n) => (activityPage = n)}
  />
</Modal>
