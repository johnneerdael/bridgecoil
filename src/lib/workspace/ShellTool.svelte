<script lang="ts">
  import { onDestroy, tick, untrack } from "svelte";
  import { errorMessage, type Device } from "../model";
  import ActionReview from "./ActionReview.svelte";
  import {
    targetOf,
    type FileListing,
    type LaunchableApp,
    type Operation,
    type Platform,
    type Review,
  } from "./client";
  import Icon from "./Icon.svelte";
  import {
    commandOf,
    complete,
    effectOf,
    keep,
    knownRead,
    matches,
    outputOf,
    pickValue,
    pickWord,
    remembered,
    scrollback,
    stepAfter,
    textOf,
    visible,
    warningsOf,
    type Entry,
    type Helper,
    type Line,
    type ListSource,
    type Step,
    type Word,
  } from "./shell-line";
  import { root } from "./shell-words";

  type Row = { value: string; about: string };
  let {
    device,
    api,
    reload = 0,
  }: { device: Device; api: Platform; reload?: number } = $props();
  const target = $derived(targetOf(device));
  const context = $derived({ user: device.profileId });

  let mode = $state<"normal" | "advanced">("normal");
  let typed = $state("");
  let line = $state<Line>([]);
  let open = $state(false);
  let search = $state("");
  let draft = $state("");
  let custom = $state(false);
  let folder = $state("");
  let running = $state(false);
  let error = $state("");
  let review = $state<Review | null>(null);
  let entries = $state<Entry[]>(untrack(() => scrollback(device.deviceId)));
  let lists = $state<Record<string, Row[]>>({});
  let labels = $state<Record<string, string>>({});
  let loading = $state(false);
  let listError = $state("");
  let files = $state<FileListing | null>(null);
  let installed = $state<Set<string> | undefined>();
  let windowEl = $state<HTMLElement>();
  let fieldEl = $state<HTMLElement>();
  let serial = 0;

  const step = $derived(stepAfter(line, root));
  const ready = $derived(complete(line, root));
  const helper = $derived<Helper | undefined>(
    step?.kind === "text"
      ? step.helper
      : custom && step && "other" in step
        ? step.other
        : undefined,
  );
  const draftValid = $derived(
    !!draft.trim() && (!helper?.pattern || helper.pattern.test(draft.trim())),
  );
  const shown = $derived(textOf(line));
  const warnings = $derived(warningsOf(line));
  const canRun = $derived(
    !running &&
      (mode === "advanced"
        ? !!typed.trim()
        : ready || (!!helper && draftValid && !onward(step))),
  );
  /** Whether typing the value finishes the line, so Run can take it straight away. */
  function onward(from: Step | null) {
    const next = from?.next;
    return typeof next === "function" ? next() : next;
  }

  const listKey = (source: ListSource | "commands", argument: string | null) =>
    `${source}:${argument ?? ""}`;
  const argumentFor = (from: Step | null) =>
    from?.kind === "list" && from.argument
      ? (remembered(line, from.argument) ?? from.argument)
      : null;

  /** Reads a list in the background, so the dropdown is ready and always current. */
  async function refreshList(source: ListSource | "commands", argument: string | null) {
    const key = listKey(source, argument);
    const rows = await api.query<Row[]>(target, {
      kind: "shellList",
      source,
      argument,
    });
    lists = { ...lists, [key]: rows };
    return rows;
  }
  async function refreshPackages() {
    const apps = await api
      .query<LaunchableApp[]>(target, { kind: "launchableApps" })
      .catch(() => []);
    labels = Object.fromEntries(
      apps.map((a) => [a.component.split("/")[0], a.label]),
    );
  }
  let prepared = -1;
  $effect(() => {
    if (prepared === reload) return;
    prepared = reload;
    void refreshList("commands", null)
      .then((rows) => (installed = new Set(rows.map((row) => row.value))))
      .catch(() => {});
    void refreshList("packages", null).catch(() => {});
    void refreshPackages();
  });

  async function loadStep(next: Step | null) {
    search = "";
    draft = "";
    custom = false;
    listError = "";
    if (next?.kind === "path") {
      folder = "";
      files = null;
    }
    if (next?.kind !== "list") return;
    const argument = argumentFor(next);
    const ticket = ++serial;
    loading = !lists[listKey(next.source, argument)];
    try {
      await refreshList(next.source, argument);
    } catch (e) {
      if (ticket === serial) listError = errorMessage(e);
    } finally {
      if (ticket === serial) loading = false;
    }
  }
  async function openFolder(path: string) {
    const ticket = ++serial;
    loading = true;
    listError = "";
    search = "";
    try {
      const listing = await api.query<FileListing>(target, {
        kind: "files",
        path,
      });
      if (ticket !== serial) return;
      files = listing;
      folder = listing.path;
      if (listing.status === "denied")
        listError = "Android doesn't let the shell read this folder.";
      else if (listing.status !== "ok") listError = "This folder isn't there.";
    } catch (e) {
      if (ticket === serial) listError = errorMessage(e);
    } finally {
      if (ticket === serial) loading = false;
    }
  }

  const wordRows = $derived<Word[]>(
    step?.kind === "words"
      ? step.words.filter((word) =>
          visible(word, device.sdk, line.length ? undefined : installed),
        )
      : step?.kind === "list"
        ? (step.extra ?? []).filter((word) => visible(word, device.sdk))
        : [],
  );
  const liveRows = $derived.by<{ word: string; about: string; value: string }[]>(() => {
    if (step?.kind !== "list") return [];
    const taken = new Set(wordRows.map((row) => row.word));
    const packages = step.source.endsWith("ackages") && step.source !== "termuxPackages";
    return (lists[listKey(step.source, argumentFor(step))] ?? [])
      .filter((row) => !taken.has(row.value))
      .map((row) => ({
        word: row.value,
        value: row.value,
        about:
          row.about ||
          (packages ? labels[row.value] : step.describe?.[row.value]) ||
          "",
      }))
      .sort((a, b) => Number(!a.about) - Number(!b.about));
  });
  const fileRows = $derived(
    step?.kind === "path" && files?.status === "ok"
      ? files.entries.filter(
          (entry) => !step.folders || entry.kind === "folder",
        )
      : [],
  );
  const places = [
    ["/sdcard/", "Shared storage."],
    ["/sdcard/Download/", "Downloads."],
    ["/data/local/tmp/", "The shell's own folder."],
    ["/system/", "Android itself. Read only."],
    ["/", "The top of the TV's storage."],
  ];
  const wordsFound = $derived(matches(wordRows, search));
  const liveFound = $derived(matches(liveRows, search));
  const filesFound = $derived(
    fileRows.filter((entry) =>
      entry.name.toLowerCase().includes(search.trim().toLowerCase()),
    ),
  );
  const rowCount = $derived(wordRows.length + liveRows.length + fileRows.length);
  const groupOf = (index: number) => {
    const group = wordsFound[index].group;
    return group !== wordsFound[index - 1]?.group ? group : undefined;
  };

  function advance(next: Line) {
    line = next;
    const following = stepAfter(next, root);
    open = !!following;
    void loadStep(following);
    // A search box must not keep the keyboard up over the next list.
    if (following?.kind !== "text" && document.activeElement instanceof HTMLElement)
      document.activeElement.blur();
    void tick().then(() => {
      if (fieldEl) fieldEl.scrollLeft = fieldEl.scrollWidth;
    });
  }
  function chooseWord(word: Word) {
    if (step) advance(pickWord(line, step, word, context));
  }
  function chooseValue(value: string) {
    if (step) advance(pickValue(line, step, value));
  }
  function commitDraft() {
    if (!step || !draftValid) return;
    const value = draft.trim();
    draft = "";
    advance(pickValue(line, step, value));
  }
  function back() {
    advance(line.slice(0, -1));
    open = true;
  }
  function clear() {
    advance([]);
    open = false;
  }
  function toggle() {
    open = !open;
    if (open) void loadStep(step);
  }

  function record(command: string, output: string, failed: boolean, note?: string) {
    entries = keep(device.deviceId, [
      ...entries,
      { id: Date.now() + entries.length, command, output, failed, note },
    ]);
    const added = entries.at(-1)!.id;
    void tick().then(() => {
      const start = windowEl?.querySelector(`[data-entry="${added}"]`);
      if (!windowEl || !start) return;
      windowEl.scrollTop +=
        start.getBoundingClientRect().top - windowEl.getBoundingClientRect().top;
    });
  }
  function finish(command: string, result: Operation) {
    const code = result.error?.match(/exit code (\d+)/)?.[1];
    const failed = result.status === "failed" || result.status === "unknown";
    record(
      command,
      outputOf(result.output),
      failed,
      code ? `Exit code ${code}` : (result.error ?? undefined),
    );
  }
  async function send(command: string, reads: boolean) {
    running = true;
    error = "";
    open = false;
    try {
      const prepared = await api.prepare(target, { kind: "shell", command });
      if (!reads) {
        review = prepared;
        return;
      }
      finish(command, await api.apply(prepared.id));
    } catch (e) {
      record(command, "", true, errorMessage(e));
    } finally {
      running = false;
    }
  }
  async function run() {
    if (!canRun) return;
    if (mode === "advanced")
      return send(typed.trim(), knownRead(typed, root, device.sdk));
    const final = helper && draftValid ? pickValue(line, step!, draft.trim()) : line;
    draft = "";
    line = final;
    await send(commandOf(final), effectOf(final) === "read");
    if (effectOf(final) === "read") clear();
  }
  function reviewed(result: Operation) {
    finish(review?.commands[0] ?? "", result);
    if (mode === "normal") clear();
  }
  function closeReview() {
    if (review) void api.discard(review.id).catch(() => {});
    review = null;
  }
  async function look(find: NonNullable<Helper["find"]>) {
    await send(find.command, true);
  }
  function recall(entry: Entry) {
    mode = "advanced";
    typed = entry.command;
  }
  async function copy() {
    const text = entries
      .map((entry) => `$ ${entry.command}\n${entry.output}`)
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      error = "This device didn't allow copying.";
    }
  }
  function wipe() {
    entries = keep(device.deviceId, []);
  }
  onDestroy(() => {
    if (review) void api.discard(review.id).catch(() => {});
  });
</script>

<div class="shell-tool">
  <div class="segmented shell-modes">
    <button class:chosen={mode === "normal"} onclick={() => (mode = "normal")}
      >Normal</button
    ><button
      class:chosen={mode === "advanced"}
      onclick={() => {
        mode = "advanced";
        open = false;
      }}>Advanced</button
    >
  </div>

  <form
    class="shell-bar"
    onsubmit={(e) => {
      e.preventDefault();
      void run();
    }}
  >
    {#if mode === "advanced"}<input
        class="shell-input"
        aria-label="Command"
        bind:value={typed}
        maxlength={8192}
        placeholder="Type a command"
        autocapitalize="off"
        autocomplete="off"
        spellcheck={false}
        enterkeyhint="go"
      />
    {:else if helper}<div class="shell-field" bind:this={fieldEl}>
        {#if shown}<span class="shell-words">{shown}</span>{/if}<input
          class="shell-input"
          aria-label={helper.label}
          bind:value={draft}
          placeholder={helper.example}
          autocapitalize="off"
          autocomplete="off"
          spellcheck={false}
          enterkeyhint="done"
          onkeydown={(e) => {
            if (e.key === "Enter" && onward(step)) {
              e.preventDefault();
              commitDraft();
            }
          }}
        />
      </div>
    {:else}<button
        type="button"
        class="shell-field"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label="Command"
        bind:this={fieldEl}
        onclick={toggle}
        >{#if shown}<span class="shell-words">{shown}</span>{:else}<span
            class="shell-placeholder">Tap to pick a command</span
          >{/if}</button
      >{/if}
    {#if mode === "normal" && line.length}<button
        type="button"
        class="icon-button"
        aria-label="Remove the last word"
        onclick={back}><Icon name="arrow-left" /></button
      >{/if}
    <button class="primary shell-run" disabled={!canRun}
      >{running ? "Running…" : "Run"}</button
    >
  </form>

  {#if mode === "normal"}
    {#each warnings as warning}<p class="risk-note" role="note">
        {warning}
      </p>{/each}
    {#if helper}<div class="shell-helper">
        <strong>{helper.label}</strong>
        <p>{helper.about}</p>
        <p class="subtle">For example: <code>{helper.example}</code></p>
        {#if draft.trim() && !draftValid}<p class="error-text" role="alert">
            That doesn't look right. It should look like the example.
          </p>{/if}
        <div class="shell-helper-actions">
          {#if helper.find}{@const find = helper.find}<button
              type="button"
              class="secondary"
              disabled={running}
              onclick={() => look(find)}>{find.label}</button
            >{/if}
          {#if onward(step)}<button
              type="button"
              class="primary"
              disabled={!draftValid}
              onclick={commitDraft}>Next word</button
            >{/if}
          {#if custom}<button
              type="button"
              class="secondary"
              onclick={() => (custom = false)}>Back to the list</button
            >{/if}
        </div>
      </div>
    {/if}
  {/if}

  <div class="shell-stage">
    {#if mode === "normal" && !helper && open && step}<div class="shell-menu">
        <div class="shell-menu-head">
          <strong>{step.prompt}</strong>
          {#if ready}<button
              type="button"
              class="primary"
              disabled={!canRun}
              onclick={run}>Run it now</button
            >{/if}
        </div>
        {#if rowCount > 8}<input
            type="search"
            aria-label="Search this list"
            placeholder="Search"
            bind:value={search}
            autocapitalize="off"
            autocomplete="off"
            spellcheck={false}
          />{/if}
        <div class="shell-rows" role="listbox" aria-label={step.prompt}>
          {#if step.kind === "path"}
            {#if !folder}
              {#each places as [place, about]}<button
                  type="button"
                  class="shell-row"
                  role="option"
                  aria-selected="false"
                  onclick={() => openFolder(place)}
                  ><code>{place}</code><small>{about}</small></button
                >{/each}
            {:else}<button
                type="button"
                class="shell-row"
                role="option"
                aria-selected="false"
                onclick={() => {
                  const up = folder.replace(/\/+$/, "").replace(/[^/]*$/, "");
                  void openFolder(up || "/");
                }}><code>..</code><small>Up one folder.</small></button
              >
              {#if step.folders}<button
                  type="button"
                  class="shell-row"
                  role="option"
                  aria-selected="false"
                  onclick={() => chooseValue(folder)}
                  ><code>{folder}</code><small>Use this folder.</small></button
                >{/if}
              {#each filesFound as entry (entry.path)}<button
                  type="button"
                  class="shell-row"
                  role="option"
                  aria-selected="false"
                  onclick={() =>
                    entry.kind === "folder"
                      ? openFolder(entry.path)
                      : chooseValue(entry.path)}
                  ><code>{entry.name}{entry.kind === "folder" ? "/" : ""}</code
                  ><small
                    >{entry.kind === "folder" ? "Open this folder." : "Use this file."}</small
                  ></button
                >{/each}
            {/if}
          {:else}
            {#each wordsFound as row, index (row.word + index)}
              {#if groupOf(index)}<p class="shell-group">{groupOf(index)}</p>{/if}
              <button
                type="button"
                class="shell-row"
                role="option"
                aria-selected="false"
                onclick={() => chooseWord(row)}
                ><code>{row.word}</code><small>{row.about}</small
                >{#if row.effect === "change"}<em class="shell-tag"
                    >Changes the TV</em
                  >{:else if row.effect === "danger"}<em
                    class="shell-tag danger-text">Be careful</em
                  >{/if}</button
              >
            {/each}
            {#each liveFound.slice(0, 300) as row (row.value)}<button
                type="button"
                class="shell-row"
                role="option"
                aria-selected="false"
                onclick={() => chooseValue(row.value)}
                ><code>{row.word}</code>{#if row.about}<small>{row.about}</small
                  >{/if}</button
              >{/each}
            {#if liveFound.length > 300}<p class="subtle">
                Search to narrow down {liveFound.length} more.
              </p>{/if}
          {/if}
          {#if loading}<p class="subtle" role="status">Reading the TV…</p>{/if}
          {#if listError}<p class="error-text" role="alert">{listError}</p>{/if}
          {#if !loading && step.kind === "list" && !rowCount && !listError}<p
              class="subtle"
            >
              The TV didn't list anything here.
            </p>{/if}
          {#if "other" in step && step.other}<button
              type="button"
              class="shell-row"
              role="option"
              aria-selected="false"
              onclick={() => {
                custom = true;
                draft = "";
              }}
              ><code>Type it myself</code><small>Enter a value that isn't listed.</small
              ></button
            >{/if}
        </div>
      </div>{/if}
  <section class="shell-window" aria-label="Output">
    <div class="shell-window-actions">
      <button
        type="button"
        class="icon-button"
        aria-label="Copy the output"
        disabled={!entries.length}
        onclick={copy}><Icon name="copy" /></button
      ><button
        type="button"
        class="icon-button"
        aria-label="Clear the output"
        disabled={!entries.length}
        onclick={wipe}><Icon name="close" /></button
      >
    </div>
    <!-- Keyboard users need to focus the scrolling region to use arrow/Page keys. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="shell-output" bind:this={windowEl} tabindex="0">
      {#if running}<p class="subtle" role="status">Running on the TV…</p>{/if}
      {#if error}<p class="error-text" role="alert">{error}</p>{/if}
      {#if !entries.length && !running}<p class="shell-empty">
          Output appears here.
        </p>{/if}
      {#each entries as entry (entry.id)}<div data-entry={entry.id}>
          <button
            type="button"
            class="shell-prompt"
            aria-label={`Edit ${entry.command}`}
            onclick={() => recall(entry)}>$ {entry.command}</button
          >
          {#if entry.output}<pre>{entry.output}</pre>{/if}
          {#if entry.note}<p class={entry.failed ? "error-text" : "subtle"}>
              {entry.note}
            </p>{:else if !entry.output}<p class="subtle">
              The command finished with no output.
            </p>{/if}
        </div>{/each}
    </div>
  </section>
  </div>
</div>

<ActionReview
  {review}
  {api}
  deviceName={device.model}
  onclose={closeReview}
  onresult={reviewed}
/>
