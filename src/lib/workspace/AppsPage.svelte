<script lang="ts">
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
  let {
    device,
    api,
    onreview,
    onbulk,
    oninstall,
    filter = $bindable("all"),
    search = $bindable(""),
    submitted = $bindable(""),
    page = $bindable(0),
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
    onbulk: (r: BulkReview) => void;
    oninstall: () => void;
    filter?: string;
    search?: string;
    submitted?: string;
    page?: number;
  } = $props();
  const pageSize = 3;
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
  type AppPage = Page<AppMetadata> & { metadataNote?: string | null };
  let result = $state<AppPage>({
    items: [],
    total: 0,
    page: 0,
    pageSize,
  });
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
  $effect(() => {
    const target = targetOf(device);
    const q = {
      kind: "packages" as const,
      filter,
      search: submitted,
      page,
      pageSize,
    };
    refresh;
    let cancelled = false;
    busy = true;
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
  async function inspect(packageId: string) {
    busy = true;
    error = "";
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
  async function plan(action: Action) {
    busy = true;
    error = "";
    try {
      const r = await api.prepare(targetOf(device), action);
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
      error =
        r.error ??
        (r.status === "sent" ? "Launch sent. Check the TV." : r.output);
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
  >
{/snippet}
<section class="feature-page">
  <div class="toolbar">
    <label class="sr-only" for="app-filter">App filter</label><select
      id="app-filter"
      bind:value={filter}
      onchange={() => (page = 0)}
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
      page = 0;
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
        onclick={reviewBulk}>Review {selected.length} apps</button
      >
    </div>
  {:else}<p class="subtle">
      Profile {device.profileId} · {busy
        ? "Reading apps…"
        : `${result.total} apps`}{result.metadataNote
        ? " · Some app details unavailable"
        : ""}
    </p>{/if}
  {#if error}<p class="error-text" role="status">{error}</p>{/if}
  <div class="row-list">
    {#each result.items as app}
      {#if selecting}<label class="checkbox list-row bulk-choice"
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
    {#if !busy && !result.items.length}<p class="empty-caption">
        No apps match this view.
      </p>{/if}
  </div>
  <Pager
    {page}
    total={result.total}
    size={pageSize}
    {busy}
    onpage={(n) => (page = n)}
  />
</section>
<Modal open={!!detail} title="Application" onclose={() => (detail = null)}>
  {#if detail}<h3 class="break-word">
      {detail.metadata?.label || detail.package}
    </h3>
    {#if detail.metadata?.label}<p class="subtle break-word">
        {detail.package}
      </p>{/if}
    <div class="segmented">
      {#each ["overview", "actions", "permissions", "raw"] as section}<button
          class:chosen={tab === section}
          onclick={() => (tab = section)}>{section}</button
        >{/each}
    </div>
    {#if tab === "overview"}<dl>
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
      {#each detail.activities.slice(0, 3) as component}<button
          class="secondary"
          disabled={busy}
          onclick={() => launch(component)}
          >Launch {component.split("/")[1]}</button
        >{/each}{#if !detail.activities.length}<p class="subtle">
          No launch activity reported.
        </p>{/if}
    {:else if tab === "actions"}<div class="action-grid">
        {#each actions.slice(actionPage * 4, actionPage * 4 + 4) as action}<button
            class="secondary"
            disabled={busy}
            onclick={() =>
              detail &&
              plan({
                kind: "package",
                package: detail.package,
                action: action.id,
              })}>{action.label}</button
          >{/each}
      </div>
      <Pager
        page={actionPage}
        total={actions.length}
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
    {:else}<Output
        text={`Signing certificate SHA-256:\n${detail.metadata?.signers?.join("\n") || "Not reported"}\n\n${detail.raw}`}
      />{/if}
    {#if error}<p class="error-text" role="alert">{error}</p>{/if}
  {/if}
</Modal>
