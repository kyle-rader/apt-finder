// Thin wrapper around the backend JSON API.

async function req(method, url, body) {
  const opts = { method, headers: {} };
  if (body !== undefined) {
    opts.headers["Content-Type"] = "application/json";
    opts.body = JSON.stringify(body);
  }
  const res = await fetch(url, opts);
  if (res.status === 204) return null;
  const text = await res.text();
  const data = text ? JSON.parse(text) : null;
  if (!res.ok) {
    throw new Error((data && data.error) || `Request failed (${res.status})`);
  }
  return data;
}

export const api = {
  getState: () => req("GET", "/api/state"),
  geocode: (q) => req("GET", `/api/geocode?q=${encodeURIComponent(q)}`),
  addApartment: (body) => req("POST", "/api/apartments", body),
  deleteApartment: (id) => req("DELETE", `/api/apartments/${id}`),
  addStat: (body) => req("POST", "/api/stats", body),
  deleteStat: (id) => req("DELETE", `/api/stats/${id}`),
  recomputeStat: (id) => req("POST", `/api/stats/${id}/recompute`, {}),
};
