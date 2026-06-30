<script>
  import { onMount } from "svelte";
  import L from "leaflet";
  import { store, colorFor } from "./store.svelte.js";

  // Toggle radius rings + which walking stat's target(s) to highlight.
  let { rings, selectedStat } = $props();

  const MILES = [
    { mi: 0.5, m: 804.672 },
    { mi: 1, m: 1609.344 },
    { mi: 2, m: 3218.688 },
  ];

  let mapEl;
  let map;
  let layer; // group holding everything we redraw

  onMount(() => {
    map = L.map(mapEl).setView([47.6151, -122.3194], 14); // Cal Anderson Park
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      maxZoom: 19,
      attribution: "© OpenStreetMap contributors",
    }).addTo(map);
    layer = L.layerGroup().addTo(map);
    return () => map.remove();
  });

  function aptIcon(color, label) {
    return L.divIcon({
      className: "",
      html: `<div style="background:${color};width:18px;height:18px;border-radius:50%;border:2px solid #04121f;box-shadow:0 0 0 2px ${color}66" title="${label}"></div>`,
      iconSize: [18, 18],
      iconAnchor: [9, 9],
    });
  }

  function targetIcon() {
    return L.divIcon({
      className: "",
      html: `<div style="font-size:22px;line-height:22px">🎯</div>`,
      iconSize: [22, 22],
      iconAnchor: [11, 11],
    });
  }

  // Redraw whenever data, rings toggle, or the selected stat changes.
  $effect(() => {
    if (!map || !layer) return;
    // Touch reactive deps so the effect re-runs.
    const apartments = store.apartments;
    const showRings = rings;
    const stat = selectedStat;

    layer.clearLayers();
    const points = [];

    apartments.forEach((apt, i) => {
      const color = colorFor(i);
      const ll = [apt.lat, apt.lng];
      points.push(ll);

      L.marker(ll, { icon: aptIcon(color, apt.name) })
        .bindPopup(`<b>${apt.name}</b><br>${apt.address}`)
        .addTo(layer);

      if (showRings) {
        MILES.forEach(({ m }) => {
          L.circle(ll, {
            radius: m,
            color,
            weight: 1,
            opacity: 0.5,
            fillOpacity: 0.04,
          }).addTo(layer);
        });
      }

      // Connector lines to the selected walking stat's target.
      if (stat && stat.kind === "walking" && stat.target_lat != null) {
        L.polyline([ll, [stat.target_lat, stat.target_lng]], {
          color,
          weight: 2,
          dashArray: "4 6",
          opacity: 0.7,
        }).addTo(layer);
      }
    });

    if (stat && stat.kind === "walking" && stat.target_lat != null) {
      const t = [stat.target_lat, stat.target_lng];
      points.push(t);
      L.marker(t, { icon: targetIcon() })
        .bindPopup(`<b>🎯 ${stat.name}</b><br>${stat.target_label ?? ""}`)
        .addTo(layer);
    }

    if (points.length > 0) {
      map.fitBounds(L.latLngBounds(points).pad(0.25), { maxZoom: 15 });
    }
  });
</script>

<div id="map" bind:this={mapEl}></div>
