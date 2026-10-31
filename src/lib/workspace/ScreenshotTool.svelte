<script lang="ts">
  import { onMount } from "svelte";
  import { errorMessage, type Device } from "../model";
  import Modal from "../Modal.svelte";
  import {
    targetOf,
    type Platform,
    type RecordingState,
    type RecordingStep,
  } from "./client";
  let { device, api }: { device: Device; api: Platform } = $props();
  const lengths = [
    { seconds: 15, label: "15 seconds" },
    { seconds: 30, label: "30 seconds" },
    { seconds: 60, label: "1 minute" },
    { seconds: 180, label: "3 minutes" },
  ];
  let image = $state("");
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
  async function attempt(work: () => Promise<void>) {
    busy = true;
    error = "";
    notice = "";
    try {
      await work();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
    }
  }
  const take = () =>
    attempt(async () => {
      const r = await api.query<{ png: string }>(targetOf(device), {
        kind: "screenshot",
      });
      image = r.png;
    });
  const saveImage = () =>
    attempt(async () => {
      if (await api.saveFile(`tv-${stamp()}.png`, { base64: image }))
        notice = "Screenshot saved.";
    });
  const step = (next: RecordingStep) =>
    api.query<RecordingState>(targetOf(device), {
      kind: "recording",
      step: next,
    });
  // A recording outlives this screen, so pick up whatever the TV is doing.
  onMount(() => {
    void step({ kind: "status" })
      .then((state) => {
        if (state.running) recording = true;
        else if (state.size) finished = state;
      })
      .catch(() => {});
  });
  function done(state: RecordingState) {
    recording = false;
    if (state.size) finished = state;
    else error = "The TV didn't produce a recording.";
  }
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
          if (recording && !state.running) done(state);
        })
        // One missed check isn't the end of a recording, and Stop has to stay available.
        .catch((e) => {
          error = errorMessage(e);
        });
    }, 2000);
    return () => clearInterval(timer);
  });
  const saveRecording = () =>
    attempt(async () => {
      if (!finished) return;
      saving = 0;
      try {
        const saved = await api.saveTvFile(
          device.id,
          finished.path,
          `tv-${stamp()}.mp4`,
          ({ received, total }) =>
            (saving = total ? Math.round((received / total) * 100) : 0),
        );
        if (!saved) return;
      } finally {
        saving = null;
      }
      await step({ kind: "discard" });
      finished = null;
      notice = "Recording saved and removed from the TV.";
    });
  const discard = () =>
    attempt(async () => {
      await step({ kind: "discard" });
      finished = null;
    });
</script>

{#if error}<p class="error-text" role="alert">{error}</p>{/if}
{#if notice}<p class="subtle" role="status">{notice}</p>{/if}
<p class="subtle">
  Streaming apps usually appear black, because they protect their video.
</p>
<div class="capture-actions">
  <button class="primary" disabled={busy || recording} onclick={take}
    >{busy ? "Working…" : "Take a screenshot"}</button
  >
  {#if image}<button class="secondary" disabled={busy} onclick={saveImage}
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
