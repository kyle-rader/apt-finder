<script>
  import { onMount } from "svelte";
  import { store, startPolling } from "./lib/store.svelte.js";
  import Map from "./lib/Map.svelte";
  import ApartmentPanel from "./lib/ApartmentPanel.svelte";
  import StatsPanel from "./lib/StatsPanel.svelte";
  import ComparisonTable from "./lib/ComparisonTable.svelte";

  let rings = $state(true);
  let selectedStatId = $state(null);

  let selectedStat = $derived(
    store.stats.find((s) => s.id === selectedStatId) ?? null
  );

  onMount(() => startPolling());
</script>

<div class="app">
  <aside class="sidebar">
    <div class="row" style="justify-content:space-between">
      <h1 style="font-size:18px">🏙️ Apartment Finder</h1>
    </div>
    <label class="row" style="gap:6px;cursor:pointer">
      <input type="checkbox" bind:checked={rings} style="width:auto" />
      <span>Show ½ / 1 / 2 mi rings</span>
    </label>

    <ApartmentPanel />
    <StatsPanel bind:selectedStatId />
  </aside>

  <main class="main">
    <Map {rings} {selectedStat} />
    <ComparisonTable />
  </main>
</div>
