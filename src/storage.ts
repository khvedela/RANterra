import { openDB } from "idb";
import type { Scenario } from "./types";
const db = openDB("ranterra", 1, {
  upgrade(db) {
    db.createObjectStore("scenarios");
    db.createObjectStore("assets");
  },
});
export async function saveScenario(s: Scenario) {
  await (await db).put("scenarios", s, "current");
}
export async function loadScenario(): Promise<Scenario | undefined> {
  return (await db).get("scenarios", "current");
}
export async function cacheGet<T>(key: string): Promise<T | undefined> {
  return (await db).get("assets", key);
}
export async function cachePut(key: string, value: unknown) {
  await (await db).put("assets", value, key);
}
