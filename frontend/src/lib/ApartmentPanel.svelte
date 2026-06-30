<script>
  import { api } from "./api.js";
  import { store, colorFor, refresh } from "./store.svelte.js";

  let address = $state("");
  let name = $state("");
  let sourceUrl = $state("");
  let notes = $state("");
  let saving = $state(false);
  let error = $state("");

  async function add() {
    if (!address.trim()) return;
    saving = true;
    error = "";
    try {
      await api.addApartment({
        address,
        name: name || undefined,
        source_url: sourceUrl || undefined,
        notes: notes || undefined,
      });
      address = name = sourceUrl = notes = "";
      await refresh();
    } catch (e) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function remove(id) {
    await api.deleteApartment(id);
    await refresh();
  }
</script>

<div class="card">
  <h2>Search list</h2>

  {#if store.apartments.length === 0}
    <p class="muted">No apartments yet. Add one by address below.</p>
  {/if}

  {#each store.apartments as apt, i (apt.id)}
    <div class="list-item">
      <div class="row" style="align-items:flex-start">
        <span class="swatch" style="background:{colorFor(i)}"></span>
        <div>
          <div>{apt.name}</div>
          <div class="muted">{apt.address}</div>
          {#if apt.source_url}
            <a class="muted" href={apt.source_url} target="_blank" rel="noreferrer"
              >listing ↗</a
            >
          {/if}
        </div>
      </div>
      <button class="danger-link" onclick={() => remove(apt.id)} title="Remove">✕</button>
    </div>
  {/each}

  <label>Address *</label>
  <input bind:value={address} placeholder="e.g. 1620 Broadway, Seattle, WA" />
  <label>Name (optional)</label>
  <input bind:value={name} placeholder="defaults to the matched address" />
  <label>Listing URL (optional)</label>
  <input bind:value={sourceUrl} placeholder="Zillow / building website" />
  <label>Notes (optional)</label>
  <input bind:value={notes} placeholder="studio, $1800, etc." />

  {#if error}<div class="error">{error}</div>{/if}

  <div style="margin-top:10px">
    <button class="primary" onclick={add} disabled={saving || !address.trim()}>
      {saving ? "Adding…" : "Add apartment"}
    </button>
  </div>
</div>
