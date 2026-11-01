<script lang="ts">
  import { onMount } from "svelte";
  import { errorMessage, type Device } from "../model";
  import Modal from "../Modal.svelte";
  import {
    targetOf,
    type Platform,
    type RecordingState,
    type RecordingStep,
    type SavedCapture,
  } from "./client";
  let { device, api }: { device: Device; api: Platform } = $props();
  const lengths = [
    { seconds: 15, label: "15 seconds" },
    { seconds: 30, label: "30 seconds" },
    { seconds: 60, label: "1 minute" },
    { seconds: 180, label: "3 minutes" },
  ];
  let image = $state("");
  let imageKept = $state(false);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  let seconds = $state(30);
  let recording = $state(false);
  let finished = $state<RecordingState | null>(null);
  let saving = $state<number | null>(null);
  // Local time, because this is what people see in their file list.
  const stamp = () => {
    const now = new Date();
    return new Date(now.getTime() - now.getTimezoneOffset() * 60000)
      .toISOString()
      .slice(0, 19)
      .replace(/[-:]/g, "")
      .replace("T", "-");
  };
  async function attempt(work: () => Promise<void>, keepNotice = false) {
    busy = true;
    error = "";
    if (!keepNotice) {
      notice = "";
      saved = null;
    }
    try {
      await work();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  let saved = $state<{ uri: string; mime: string; label: string } | null>(null);
  const placed = (to: SavedCapture) =>
    to.location ? ` to your phone's ${to.location} folder` : "";
  async function keepImage() {
    const to = await api.saveCapture(`tv-${stamp()}.png`, image);
    if (!to) return;
    imageKept = true;
    notice = `Screenshot saved${placed(to)}.`;
    saved = to.uri ? { uri: to.uri, mime: "image/png", label: "screenshot" } : null;
  }
  const openSaved = () =>
    attempt(async () => {
      if (saved) await api.openCapture(saved.uri, saved.mime);
    }, true);
  // A screenshot taken from the phone belongs on the phone, so it goes there straight away.
  const take = () =>
    attempt(async () => {
      const r = await api.query<{ png: string }>(targetOf(device), {
        kind: "screenshot",
      });
      image = r.png;
      imageKept = false;
      await keepImage();
    });
  const saveImage = () => attempt(keepImage);
  const step = (next: RecordingStep) =>
    api.query<RecordingState>(targetOf(device), {
      kind: "recording",
      step: next,
    });
  // It leaves the TV only once it's on this device, so a failed copy can be tried again.
  async function keepRecording() {
    if (!finished) return;
    saving = 0;
    let to: SavedCapture | null;
    try {
      to = await api.saveTvCapture(
        device.id,
        finished.path,
        `tv-${stamp()}.mp4`,
        ({ received, total }) =>
          (saving = total ? Math.round((received / total) * 100) : 0),
      );
    } finally {
      saving = null;
    }
    if (!to) return;
    await step({ kind: "discard" });
    finished = null;
    notice = `Recording saved${placed(to)} and removed from the TV.`;
    saved = to.uri ? { uri: to.uri, mime: "video/mp4", label: "recording" } : null;
  }
  async function done(state: RecordingState) {
    recording = false;
    if (!state.size) {
      error = "The TV didn't produce a recording.";
      return;
    }
    finished = state;
    await keepRecording();
  }
  // A recording outlives this screen, so pick up whatever the TV is doing.
  onMount(() => {
    void step({ kind: "status" })
      .then((state) => {
        if (state.running) recording = true;
        else if (state.size) void attempt(() => done(state));
      })
      .catch(() => {});
  });
  const start = () =>
    attempt(async () => {
      finished = null;
      await step({ kind: "start", seconds });
      recording = true;
    });
  const stop = () => attempt(async () => done(await step({ kind: "stop" })));
  $effect(() => {
    if (!recording) return;
    const timer = setInterval(() => {
      void step({ kind: "status" })
        .then((state) => {
          if (recording && !state.running) void attempt(() => done(state));
        })
        // One missed check isn't the end of a recording, and Stop has to stay available.
        .catch((e) => {
          error = errorMessage(e);
        });
    }, 2000);
    return () => clearInterval(timer);
  });
  const saveRecording = () => attempt(keepRecording);
  const discard = () =>
    attempt(async () => {
      await step({ kind: "discard" });
      finished = null;
    });
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if notice}<p class="subtle" role="status">{notice}</p>{/if}
{#if saved}<button class="secondary" disabled={busy} onclick={openSaved}
    >Open {saved.label}</button
  >{/if}
<p class="subtle">
  Streaming apps usually appear black, because they protect their video.
</p>
<div class="capture-actions">
  <button class="primary" disabled={busy || recording} onclick={take}
    >{busy ? "Working…" : "Take a screenshot"}</button
  >
  {#if image && !imageKept}<button class="secondary" disabled={busy} onclick={saveImage}
      >Save screenshot</button
    >{/if}
</div>
{#if image}<img
    class="tv-screenshot"
    src={`data:image/png;base64,${image}`}
    alt={`Screen of ${device.model}`}
  />{/if}
<div class="mapping-choice">
  <label for="recording-length">Record for</label>
  <select
    id="recording-length"
    bind:value={seconds}
    disabled={busy || recording}
  >
    {#each lengths as length}<option value={length.seconds}
        >{length.label}</option
      >{/each}
  </select>
</div>
{#if recording}<p role="status">Recording the TV screen…</p>
  <button class="primary" disabled={busy} onclick={stop}>Stop recording</button>
{:else if finished}
  <div class="capture-actions">
    <button class="primary" disabled={busy} onclick={saveRecording}
      >Save recording</button
    >
    <button class="secondary" disabled={busy} onclick={discard}>Discard</button>
  </div>
{:else}<button class="secondary" disabled={busy} onclick={start}
    >Start recording</button
  >{/if}
<Modal
  open={saving !== null}
  title="Saving recording"
  dismissible={false}
  onclose={() => {}}
>
  <p role="status">Copying… {saving}%</p>
  <button class="secondary" onclick={() => api.cancelUpload()}>Cancel</button>
</Modal>
