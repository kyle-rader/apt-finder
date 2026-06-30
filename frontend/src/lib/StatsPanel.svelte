<script>
  import { api } from "./api.js";
  import { store, refresh } from "./store.svelte.js";
  import GeoSearch from "./GeoSearch.svelte";

  let { selectedStatId = $bindable() } = $props();

  let tab = $state("walking");
  let saving = $state(false);
  let error = $state("");

  // Walking-stat form
  let walkName = $state("");
  let target = $state(null); // {label, lat, lng}

  // AI-stat form
  let aiName = $state("");
  let aiPrompt = $state("");

  function onpick(r) {
    target = r;
    if (!walkName.trim()) walkName = r.label.split(",")[0];
  }

  async function addWalking() {
    if (!walkName.trim() || !target) return;
    saving = true;
    error = "";
    try {
      await api.addStat({
        name: walkName,
        kind: "walking",
        target_label: target.label,
        target_lat: target.lat,
        target_lng: target.lng,
      });
      walkName = "";
      target = null;
      await refresh();
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function addAi() {
    if (!aiName.trim() || !aiPrompt.trim()) return;
    saving = true;
    error = "";
    try {
      await api.addStat({ name: aiName, kind: "ai", prompt: aiPrompt });
      aiName = "";
      aiPrompt = "";
      await refresh();
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function removeStat(id) {
    if (selectedStatId === id) selectedStatId = null;
    await api.deleteStat(id);
    await refresh();
  }

  async function recompute(id) {
    await api.recomputeStat(id);
    await refresh();
  }
</script>

<div class="card">
  <h2>Stats</h2>

  {#each store.stats as stat (stat.id)}
    <div class="list-item">
      <div>
        <div class="row">
          <span class="tag {stat.kind}">{stat.kind}</span>
          <strong>{stat.name}</strong>
        </div>
        {#if stat.kind === "walking"}
          <button
            class="muted"
            style="border:none;background:none;padding:2px 0"
            onclick={() =>
              (selectedStatId = selectedStatId === stat.id ? null : stat.id)}
          >
            {selectedStatId === stat.id ? "✓ shown on map" : "show on map"}
          </button>
        {:else}
          <div class="muted">{stat.prompt}</div>
        {/if}
      </div>
      <div style="display:flex;flex-direction:column;gap:2px;align-items:flex-end">
        <button class="danger-link" onclick={() => removeStat(stat.id)} title="Remove">✕</button>
        <button
          class="muted"
          style="border:none;background:none;font-size:11px"
          onclick={() => recompute(stat.id)}
          title="Re-run for all apartments">↻ recompute</button
        >
      </div>
    </div>
  {/each}

  <div class="tabs" style="margin-top:12px">
    <button class:active={tab === "walking"} onclick={() => (tab = "walking")}>
      + Walking
    </button>
    <button class:active={tab === "ai"} onclick={() => (tab = "ai")}>+ AI research</button>
  </div>

  {#if tab === "walking"}
    <label>Target location</label>
    <GeoSearch {onpick} placeholder="e.g. Paramount Theatre, Seattle" />
    {#if target}
      <div class="muted" style="margin-top:6px">🎯 {target.label}</div>
    {/if}
    <label>Stat name</label>
    <input bind:value={walkName} placeholder="e.g. Walk to Paramount" />
    <div style="margin-top:10px">
      <button
        class="primary"
        onclick={addWalking}
        disabled={saving || !walkName.trim() || !target}
      >
        {saving ? "Computing…" : "Save walking stat"}
      </button>
    </div>
    <p class="muted" style="margin-top:6px">
      Computes real walking distance + time from every apartment to this spot.
    </p>
  {:else}
    <label>Stat name</label>
    <input bind:value={aiName} placeholder="e.g. In-unit laundry?" />
    <label>Research question for the AI</label>
    <textarea
      rows="3"
      bind:value={aiPrompt}
      placeholder="e.g. Does this building have in-unit laundry, and what do recent reviews say about noise?"
    ></textarea>
    <div style="margin-top:10px">
      <button class="primary" onclick={addAi} disabled={saving || !aiName.trim() || !aiPrompt.trim()}>
        {saving ? "Starting…" : "Save AI stat"}
      </button>
    </div>
    <p class="muted" style="margin-top:6px">
      Runs your local <code>claude</code> CLI to research each apartment. Results fill in as they
      finish.
    </p>
  {/if}

  {#if error}<div class="error">{error}</div>{/if}
</div>
