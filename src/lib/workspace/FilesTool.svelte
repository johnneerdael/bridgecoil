<script lang="ts">
  import { untrack } from "svelte";
  import Modal from "../Modal.svelte";
  import { errorMessage, type Device } from "../model";
  import {
    targetOf,
    type Action,
    type FileEntry,
    type FileListing,
    type Platform,
    type Review,
    type UploadChoice,
  } from "./client";
  import Icon from "./Icon.svelte";
  import {
    breadcrumbs,
    entryIcon,
    entryNote,
    folderSummary,
    formatSize,
    internalStorage,
    joinPath,
    nameProblem,
    parentPath,
    storageRoots,
    validName,
    visibleEntries,
  } from "./files";
  let {
    device,
    api,
    reload,
    onreview,
    path = $bindable(internalStorage),
  }: {
    device: Device;
    api: Platform;
    reload: number;
    onreview: (r: Review) => void;
    path?: string;
  } = $props();
  let listing = $state<FileListing | null>(null);
  let busy = $state(false);
  let error = $state("");
  let actionError = $state("");
  let notice = $state("");
  let showHidden = $state(false);
  let editing = $state(false);
  let draft = $state("");
  let trail = $state<HTMLElement>();
  let refresh = $state(0);
  let menu = $state<FileEntry | null>(null);
  let moving = $state<FileEntry | null>(null);
  let naming = $state<{ entry: FileEntry | null; value: string } | null>(null);
  let upload = $state<{
    choice: UploadChoice;
    name: string;
    sent: number | null;
  } | null>(null);
  const crumbs = $derived(breadcrumbs(path));
  const all = $derived(listing?.entries ?? []);
  const entries = $derived(visibleEntries(all, showHidden));
  const hiddenCount = $derived(all.length - visibleEntries(all, false).length);
  const writable = $derived(listing?.status === "ok" && listing.writable);
  const nameTaken = (name: string, except?: FileEntry | null) =>
    all.some((e) => e.name === name && e !== except);
  const moveTarget = $derived(moving ? joinPath(path, moving.name) : "");
  const canMoveHere = $derived(
    !!moving && writable && moveTarget !== moving.path && !nameTaken(moving.name),
  );
  const uploading = $derived(upload?.sent !== null && upload !== null);
  $effect(() => {
    const target = targetOf(device),
      currentPath = path;
    reload;
    refresh;
    let cancelled = false;
    busy = true;
    error = "";
    untrack(() => {
      if (listing?.path !== currentPath) listing = null;
    });
    void (async () => {
      try {
        const r = await api.query<FileListing>(target, {
          kind: "files",
          path: currentPath,
        });
        if (!cancelled) {
          listing = r;
          // Later actions must use the TV's path, not what was typed.
          if (r.path !== currentPath) path = r.path;
        }
      } catch (e) {
        if (!cancelled) error = errorMessage(e);
      } finally {
        if (!cancelled) busy = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });
  $effect(() => {
    crumbs;
    if (trail) trail.scrollLeft = trail.scrollWidth;
  });
  function open(next: string) {
    path = next;
    editing = false;
    actionError = "";
    notice = "";
  }
  function startEditing() {
    draft = path;
    editing = true;
  }
  async function plan(action: Action) {
    actionError = "";
    notice = "";
    menu = null;
    naming = null;
    try {
      onreview(await api.prepare(targetOf(device), action));
      moving = null;
    } catch (e) {
      actionError = errorMessage(e);
    }
  }
  function submitName() {
    if (
      !naming ||
      !validName(naming.value) ||
      nameProblem(path, naming.value)
    )
      return;
    const { entry, value } = naming;
    void plan(
      entry
        ? {
            kind: "fileMove",
            from: entry.path,
            to: joinPath(path, value),
          }
        : { kind: "fileCreateFolder", path: joinPath(path, value) },
    );
  }
  async function chooseUpload() {
    actionError = "";
    notice = "";
    try {
      const choice = await api.chooseUpload(device.id);
      if (choice) upload = { choice, name: choice.name, sent: null };
    } catch (e) {
      actionError = errorMessage(e);
    }
  }
  async function startUpload() {
    if (!upload) return;
    const { choice, name } = upload;
    upload.sent = 0;
    try {
      await api.upload(device.id, choice.id, path, name, (sent) => {
        if (upload) upload.sent = sent;
      });
      notice = `Uploaded ${name}.`;
    } catch (e) {
      if ((e as { kind?: string }).kind !== "cancelled")
        actionError = errorMessage(e);
    }
    upload = null;
    refresh++;
  }
</script>

{#snippet row(entry: FileEntry, chevron: boolean)}
  <span class="entry-icon" class:locked={entryIcon(entry) === "lock"}
    ><Icon name={entryIcon(entry)} />{#if entry.link}<span class="entry-link"
        ><Icon name="link" size={12} /></span
      >{/if}</span
  ><span class="truncate"
    ><span class="entry-name">{entry.name}</span><small>{entryNote(entry)}</small
    ></span
  >{#if chevron}<Icon name="chevron-right" />{/if}
{/snippet}

<div class="file-roots" role="group" aria-label="Storage locations">
  {#each storageRoots as root}<button
      class="file-root"
      class:chosen={path === root.path}
      aria-pressed={path === root.path}
      onclick={() => open(root.path)}>{root.label}</button
    >{/each}
</div>
<div class="file-path">
  <button
    class="icon-button"
    aria-label="Parent folder"
    disabled={path === "/"}
    onclick={() => open(parentPath(path))}><Icon name="arrow-up" /></button
  >
  {#if editing}<form
      class="search-form"
      onsubmit={(e) => {
        e.preventDefault();
        if (draft.startsWith("/")) open(draft.replace(/\/+$/, "") || "/");
      }}
    >
      <label class="sr-only" for="remote-path">Device path</label><input
        id="remote-path"
        bind:value={draft}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      /><button class="secondary" type="submit">Open</button>
    </form>
  {:else}<nav class="file-crumbs" aria-label="Folder path" bind:this={trail}>
      {#each crumbs as crumb, index}{#if index}<Icon
            name="chevron-right"
            size={14}
          />{/if}<button
          aria-current={index === crumbs.length - 1 ? "page" : undefined}
          onclick={() => open(crumb.path)}>{crumb.label}</button
        >{/each}
    </nav>
    <button
      class="icon-button"
      aria-label="Go to a path"
      onclick={startEditing}><Icon name="edit" /></button
    >{/if}
</div>
{#if actionError}<p class="error-text" role="alert">{actionError}</p>{/if}
{#if notice}<p class="subtle" role="status">{notice}</p>{/if}
{#if error}<section class="center-state">
    <h2>Couldn't load this folder</h2>
    <p class="error-text" role="alert">{error}</p>
  </section>
{:else if !listing}<p class="subtle" role="status">Loading…</p>
{:else if listing.status === "denied"}<section class="center-state">
    <div class="state-icon" aria-hidden="true"><Icon name="lock" /></div>
    <h2>No access to this folder</h2>
    <p class="subtle">
      Android doesn't let ADB list this folder. Internal storage is usually
      open.
    </p>
  </section>
{:else if listing.status === "missing"}<section class="center-state">
    <h2>Folder not available</h2>
    <p class="subtle">It may not exist on this TV, or Android may be hiding it.</p>
  </section>
{:else if listing.status === "notDirectory"}<section class="center-state">
    <h2>This is a file, not a folder</h2>
    <p class="subtle">Choose the folder that contains it.</p>
  </section>
{:else if !all.length}<section class="center-state">
    <h2>This folder is empty</h2>
  </section>
{:else}<div class="file-summary">
    <span role="status">{folderSummary(entries)}</span>
    {#if hiddenCount}<button
        class="text-action"
        aria-pressed={showHidden}
        onclick={() => (showHidden = !showHidden)}
        >{showHidden ? "Hide" : "Show"} hidden ({hiddenCount})</button
      >{/if}
  </div>
  {#if !entries.length}<section class="center-state">
      <h2>Only hidden items here</h2>
    </section>
  {:else}<div class="row-list app-list">
      {#each entries as entry (entry.path)}
        {@const changeable =
          listing.writable && entry.kind !== "denied" && !moving}
        <div class="file-row" class:changeable>
          {#if entry.canOpen}<button
              class="list-row"
              onclick={() => open(entry.path)}
              >{@render row(entry, !changeable)}</button
            >
          {:else}<div class="list-row entry-static">
              {@render row(entry, false)}
            </div>{/if}
          {#if changeable}<button
              class="icon-button file-more"
              aria-label={`Actions for ${entry.name}`}
              onclick={() => (menu = entry)}><Icon name="more" /></button
            >{/if}
        </div>
      {/each}
      {#if listing.truncated}<p class="subtle">
          Showing the first {listing.entries.length} of {listing.total} items.
        </p>{/if}
    </div>{/if}
{/if}
{#if moving}<div class="file-moving">
    <p>Moving <strong>{moving.name}</strong></p>
    <div>
      <button class="secondary" onclick={() => (moving = null)}>Cancel</button
      ><button
        class="primary"
        disabled={!canMoveHere}
        onclick={() =>
          moving &&
          plan({ kind: "fileMove", from: moving.path, to: moveTarget })}
        >Move here</button
      >
    </div>
  </div>
{:else if writable}<div class="page-footer">
    <button
      class="secondary"
      onclick={() => (naming = { entry: null, value: "" })}
      ><Icon name="plus" />New folder</button
    ><button class="secondary" onclick={chooseUpload}
      ><Icon name="arrow-up" />Upload file</button
    >
  </div>
{:else if listing?.status === "ok"}<p class="subtle">
    This folder is read-only for ADB.
  </p>{/if}

<Modal open={!!menu} title={menu?.name ?? ""} onclose={() => (menu = null)}>
  {#if menu}{@const entry = menu}<div class="file-actions">
      <button
        class="secondary"
        onclick={() => {
          naming = { entry, value: entry.name };
          menu = null;
        }}>Rename</button
      ><button
        class="secondary"
        onclick={() => {
          moving = entry;
          menu = null;
        }}>Move</button
      ><button
        class="secondary"
        onclick={() => plan({ kind: "fileDelete", path: entry.path })}
        >Delete</button
      >
    </div>{/if}
</Modal>

<Modal
  open={!!naming}
  title={naming?.entry ? "Rename" : "New folder"}
  onclose={() => (naming = null)}
>
  {#if naming}<form
      class="file-upload"
      onsubmit={(e) => {
        e.preventDefault();
        submitName();
      }}
    >
      <label for="file-name">Name</label><input
        id="file-name"
        bind:value={naming.value}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      />
      {#if nameTaken(naming.value, naming.entry)}<p class="error-text">
          An item with this name already exists here.
        </p>{/if}
      {#if nameProblem(path, naming.value)}<p class="error-text">
          {nameProblem(path, naming.value)}
        </p>{/if}
      <button
        class="primary"
        type="submit"
        disabled={!validName(naming.value) ||
          !!nameProblem(path, naming.value) ||
          naming.value === naming.entry?.name ||
          nameTaken(naming.value, naming.entry)}>Continue</button
      >
    </form>{/if}
</Modal>

<Modal
  open={!!upload}
  title="Upload to the TV"
  dismissible={!uploading}
  onclose={() => {
    if (!uploading) upload = null;
  }}
>
  {#if upload}{@const sent = upload.sent}<div class="file-upload">
      <label for="upload-name">Name on the TV</label><input
        id="upload-name"
        bind:value={upload.name}
        disabled={sent !== null}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      />
      <p class="subtle">
        {formatSize(upload.choice.size)} to {crumbs
          .map((c) => c.label)
          .join(" / ")}
      </p>
      {#if nameTaken(upload.name)}<p class="error-text">
          An item with this name already exists here.
        </p>{/if}
      {#if nameProblem(path, upload.name)}<p class="error-text">
          {nameProblem(path, upload.name)}
        </p>{/if}
      {#if sent === null}<button
          class="primary"
          disabled={!validName(upload.name) ||
            !!nameProblem(path, upload.name) ||
            nameTaken(upload.name)}
          onclick={startUpload}>Upload</button
        >
      {:else}<progress
          max={upload.choice.size || 1}
          value={sent}
          aria-label="Upload progress"
        ></progress>
        <p class="subtle" role="status">
          {formatSize(sent)} of {formatSize(upload.choice.size)}
        </p>
        <button class="secondary" onclick={() => api.cancelUpload()}
          >Cancel upload</button
        >{/if}
    </div>{/if}
</Modal>
