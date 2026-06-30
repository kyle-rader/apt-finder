import { api } from "./api.js";

// A distinct color per apartment, reused for map markers + table swatches.
const PALETTE = [
  "#4ea1ff",
  "#6ee7b7",
  "#f0c674",
  "#ff8fab",
  "#b794f6",
  "#ff9f6b",
  "#5ad1cd",
  "#c3e88d",
];

export function colorFor(index) {
  return PALETTE[index % PALETTE.length];
}

// Shared reactive application state (Svelte 5 runes).
export const store = $state({
  apartments: [],
  stats: [],
  values: [],
  loaded: false,
});

/** Map of `${statId}:${apartmentId}` -> StatValue for quick table lookups. */
export function valueIndex() {
  const idx = new Map();
  for (const v of store.values) {
    idx.set(`${v.stat_id}:${v.apartment_id}`, v);
  }
  return idx;
}

/** True if any value is still pending (drives the poll loop cadence). */
export function hasPending() {
  return store.values.some((v) => v.status === "pending");
}

export async function refresh() {
  const data = await api.getState();
  store.apartments = data.apartments;
  store.stats = data.stats;
  store.values = data.values;
  store.loaded = true;
}

/**
 * Poll the backend while any value is pending (AI runs fill in over time),
 * otherwise idle. Returns a cleanup function.
 */
export function startPolling() {
  let timer;
  const tick = async () => {
    try {
      await refresh();
    } catch (e) {
      console.error("refresh failed", e);
    }
    timer = setTimeout(tick, hasPending() ? 2500 : 15000);
  };
  tick();
  return () => clearTimeout(timer);
}
