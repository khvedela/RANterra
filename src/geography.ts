import { config, boundedText } from "./config";
import type { Building, Point } from "./types";
import { cacheGet, cachePut } from "./storage";
export interface Place {
  display_name: string;
  lon: string;
  lat: string;
}
let lastSearch = 0;
export async function searchPlaces(query: string): Promise<Place[]> {
  if (!config.search)
    throw Error(
      "Place search is disabled. Configure a permitted geocoding provider in config.json, or enter longitude, latitude.",
    );
  if (query.length > 200) throw Error("Search query is too long");
  const key = `search:${config.search}:${query.toLowerCase().trim()}`;
  const cached = await cacheGet<Place[]>(key);
  if (cached) return cached;
  if (Date.now() - lastSearch < 1100)
    throw Error("Please wait a second between searches.");
  lastSearch = Date.now();
  const r = await fetch(
    `${config.search}?format=json&limit=5&q=${encodeURIComponent(query)}`,
    { signal: AbortSignal.timeout(15000) },
  );
  if (!r.ok)
    throw Error(
      "Place search is unavailable. Pan the map or enter longitude, latitude.",
    );
  const raw: unknown = JSON.parse(await boundedText(r, 100000));
  if (!Array.isArray(raw)) throw Error("Invalid place search response");
  const places = raw
    .slice(0, 5)
    .filter(
      (p): p is Place =>
        p &&
        typeof p.display_name === "string" &&
        p.display_name.length < 1000 &&
        Number.isFinite(Number(p.lon)) &&
        Math.abs(Number(p.lon)) <= 180 &&
        Number.isFinite(Number(p.lat)) &&
        Math.abs(Number(p.lat)) <= 85,
    );
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
  if (!config.overpass && !config.osmMap) throw Error("Building providers are disabled in config.json");
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
    const response = await fetch(config.overpass, {
      method: "POST",
      body: new URLSearchParams({ data: query }),
      signal: AbortSignal.timeout(12000),
    });
    if (!response.ok) throw Error("Overpass unavailable");
    const json = JSON.parse(await boundedText(response, 5000000)) as {
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
      if (!config.osmMap) throw Error("OSM map fallback disabled");
      const response = await fetch(
        `${config.osmMap}?bbox=${west},${south},${east},${north}`,
        { signal: AbortSignal.timeout(15000) },
      );
      if (!response.ok) throw Error("OSM map API unavailable");
      const xml = new DOMParser().parseFromString(
        await boundedText(response, 5000000),
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
