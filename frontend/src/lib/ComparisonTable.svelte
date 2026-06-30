<script>
  import { store, colorFor, valueIndex } from "./store.svelte.js";

  // Recomputed whenever values change.
  let index = $derived(valueIndex());

  function cell(statId, aptId) {
    return index.get(`${statId}:${aptId}`);
  }
</script>

<div class="table-wrap">
  {#if store.apartments.length === 0}
    <p class="muted" style="padding:12px">
      Add apartments and stats to build a comparison.
    </p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Apartment</th>
          {#each store.stats as stat (stat.id)}
            <th>
              <div class="stat-col-head">
                <span>{stat.name}</span>
                <span class="tag {stat.kind}">{stat.kind}</span>
              </div>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each store.apartments as apt, i (apt.id)}
          <tr>
            <td class="apt-name">
              <span class="swatch" style="background:{colorFor(i)};display:inline-block"></span>
              {apt.name}
            </td>
            {#each store.stats as stat (stat.id)}
              {@const v = cell(stat.id, apt.id)}
              <td>
                {#if !v}
                  <span class="muted">—</span>
                {:else if v.status === "pending"}
                  <span class="status-pending">⏳ researching…</span>
                {:else if v.status === "error"}
                  <span class="status-error" title={v.detail}>⚠ error</span>
                {:else}
                  <span class="cell-detail" title={v.detail}>{v.value_text}</span>
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
