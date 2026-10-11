import type { Building, Point } from "./types";
import { cacheGet, cachePut } from "./storage";
export interface Place {
  display_name: string;
  lon: string;
  lat: string;
}
let lastSearch = 0;
export async function searchPlaces(query: string): Promise<Place[]> {
  const key = `search:${query.toLowerCase().trim()}`;
  const cached = await cacheGet<Place[]>(key);
  if (cached) return cached;
  if (Date.now() - lastSearch < 1100)
    throw Error("Please wait a second between searches.");
  lastSearch = Date.now();
  const r = await fetch(
    `https://nominatim.openstreetmap.org/search?format=json&limit=5&q=${encodeURIComponent(query)}`,
    { signal: AbortSignal.timeout(15000) },
  );
  if (!r.ok)
    throw Error(
      "Place search is unavailable. Pan the map or enter longitude, latitude.",
    );
  const places = (await r.json()) as Place[];
  await cachePut(key, places);
  return places;
}
// Keep only bounded closed ways; Rust performs the final geometric validation.
function boundedBuildings(items: { id: string; ring: Point[] }[]): Building[] {
  let vertices = 0;
  const buildings: Building[] = [];
  for (const item of items) {
    const ring = item.ring;
    if (
      ring.length < 4 ||
      ring.length > 100 ||
      ring.some((p) => !p.every(Number.isFinite)) ||
      ring[0][0] !== ring.at(-1)![0] ||
      ring[0][1] !== ring.at(-1)![1]
    )
      continue;
    if (buildings.length >= 600 || vertices + ring.length > 15000) break;
    vertices += ring.length;
    buildings.push(item);
  }
  return buildings;
}
export async function fetchBuildings(center: Point): Promise<Building[]> {
  const key = `buildings:v2:${center.map((v) => v.toFixed(4)).join(",")}`;
  const cached = await cacheGet<Building[]>(key);
  if (cached) return cached;
  const [lon, lat] = center,
    dx = 0.006 / Math.cos((lat * Math.PI) / 180),
    dy = 0.006;
  const west = Math.max(-180, lon - dx),
    east = Math.min(180, lon + dx),
    south = Math.max(-85, lat - dy),
    north = Math.min(85, lat + dy);
  const query = `[out:json][timeout:10];way[building][building!="no"](${south},${west},${north},${east});out geom;`;
  let buildings: Building[];
  try {
    const response = await fetch("https://overpass-api.de/api/interpreter", {
      method: "POST",
      body: new URLSearchParams({ data: query }),
      signal: AbortSignal.timeout(12000),
    });
    if (!response.ok) throw Error("Overpass unavailable");
    const json = (await response.json()) as {
      elements: { id: number; geometry?: { lon: number; lat: number }[] }[];
      remark?: string;
    };
    if (json.remark || !Array.isArray(json.elements))
      throw Error("Incomplete Overpass response");
    buildings = boundedBuildings(
      json.elements.map((item) => ({
        id: `osm-way-${item.id}`,
        ring: (item.geometry ?? []).map((p) => [p.lon, p.lat] as Point),
      })),
    );
  } catch {
    // A small, explicit map-area request to OSM's read API. This is real
    // geometry, not fabricated fallback data, and uses the same bounding box.
    try {
      const response = await fetch(
        `https://api.openstreetmap.org/api/0.6/map?bbox=${west},${south},${east},${north}`,
        { signal: AbortSignal.timeout(15000) },
      );
      if (!response.ok) throw Error("OSM map API unavailable");
      const xml = new DOMParser().parseFromString(
        await response.text(),
        "application/xml",
      );
      if (xml.querySelector("parsererror")) throw Error("Malformed OSM data");
      const nodes = new Map(
        Array.from(xml.querySelectorAll("node")).map((n) => [
          n.getAttribute("id")!,
          [
            Number(n.getAttribute("lon")),
            Number(n.getAttribute("lat")),
          ] as Point,
        ]),
      );
      const items = Array.from(xml.querySelectorAll("way")).flatMap((w) => {
        const building = w
          .querySelector('tag[k="building"]')
          ?.getAttribute("v");
        if (!building || building === "no") return [];
        const refs = Array.from(w.querySelectorAll("nd")).map((n) =>
          nodes.get(n.getAttribute("ref")!),
        );
        if (refs.some((p) => !p)) return [];
        return [
          { id: `osm-way-${w.getAttribute("id")}`, ring: refs as Point[] },
        ];
      });
      buildings = boundedBuildings(items);
    } catch {
      throw Error(
        "Both OSM geometry services are unavailable. Existing geometry has been retained; try again later or import saved geometry.",
      );
    }
  }
  await cachePut(key, buildings);
  return buildings;
}
