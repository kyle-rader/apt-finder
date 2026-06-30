<script>
  import { api } from "./api.js";

  // Reusable place search. Calls `onpick({label, lat, lng})` on selection.
  let { onpick, placeholder = "Search a place or address" } = $props();

  let query = $state("");
  let results = $state([]);
  let loading = $state(false);
  let error = $state("");

  async function search() {
    if (!query.trim()) return;
    loading = true;
    error = "";
    results = [];
    try {
      results = await api.geocode(query);
      if (results.length === 0) error = "No matches found.";
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function pick(r) {
    onpick(r);
    results = [];
    query = r.label;
  }
</script>

<div class="row">
  <input
    {placeholder}
    bind:value={query}
    onkeydown={(e) => e.key === "Enter" && (e.preventDefault(), search())}
  />
  <button onclick={search} disabled={loading}>
    {loading ? "…" : "Search"}
  </button>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if results.length > 0}
  <ul class="geo-results">
    {#each results as r}
      <li>
        <button type="button" class="geo-result-btn" onclick={() => pick(r)}>{r.label}</button>
      </li>
    {/each}
  </ul>
{/if}
