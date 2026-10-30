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
  } from "./client";
  import Modal from "../Modal.svelte";
  import Output from "./Output.svelte";
  import Pager from "./Pager.svelte";
  import Icon from "./Icon.svelte";
  let {
    device,
    api,
    onreview,
    oninstall,
    filter = $bindable("all"),
    search = $bindable(""),
    submitted = $bindable(""),
    page = $bindable(0),
  }: {
    device: Device;
    api: Platform;
    onreview: (r: Review) => void;
    oninstall: () => void;
    filter?: string;
    search?: string;
    submitted?: string;
    page?: number;
  } = $props();
  let refresh = $state(0);
  let busy = $state(false);
  let error = $state("");
  type AppPage = Page<AppMetadata> & { metadataNote?: string | null };
  let result = $state<AppPage>({
    items: [],
    total: 0,
    page: 0,
    pageSize: 4,
  });
  let detail = $state<PackageDetail | null>(null);
  let tab = $state("overview");
  let actionPage = $state(0);
  let permissionPage = $state(0);
  let allPermissions = $state(false);
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
      pageSize: 4,
    };
    refresh;
    let cancelled = false;
    busy = true;
    error = "";
    result = { items: [], total: 0, page: 0, pageSize: 4 };
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
        allPermissions = false;
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
      placeholder="Search app names"
    /><button class="secondary" disabled={busy} type="submit">Search</button>
  </form>
  <p class="subtle">
    Profile {device.profileId} · {busy
      ? "Reading apps…"
      : `${result.total} apps`}{result.metadataNote
      ? " · Some app details unavailable"
      : ""}
  </p>
  {#if error}<p class="error-text" role="status">{error}</p>{/if}
  <div class="row-list">
    {#each result.items as app}<button
        class="list-row"
        disabled={busy}
        onclick={() => inspect(app.package)}
        >{#if app.icon}<img
            class="app-icon"
            src={`data:image/png;base64,${app.icon}`}
            alt=""
          />{:else}<Icon name="apps" />{/if}<span class="truncate"
          ><strong>{app.label || app.package}</strong>{#if app.label}<small
              >{app.package}</small
            >{/if}</span
        ><Icon name="chevron-right" /></button
      >{/each}
    {#if !busy && !result.items.length}<p class="empty-caption">
        No apps match this view.
      </p>{/if}
  </div>
  <Pager
    {page}
    total={result.total}
    size={4}
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
        <button
          class:chosen={!allPermissions}
          onclick={() => (allPermissions = false)}>Runtime access</button
        ><button
          class:chosen={allPermissions}
          onclick={() => (allPermissions = true)}>All permissions</button
        >
      </div>
      {#if allPermissions}<Output
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
