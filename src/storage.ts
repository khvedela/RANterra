import { openDB } from "idb";
import type { Scenario } from "./types";
let connection: ReturnType<typeof openDB> | undefined;
const db = () =>
  (connection ??= openDB("ranterra", 1, {
    upgrade(db) {
      db.createObjectStore("scenarios");
      db.createObjectStore("assets");
    },
  }).catch((error) => {
    connection = undefined;
    throw error;
  }));
export async function saveScenario(s: Scenario) {
  await (await db()).put("scenarios", s, "current");
}
export async function loadScenario(): Promise<Scenario | undefined> {
  return (await db()).get("scenarios", "current");
}
export async function cacheGet<T>(key: string): Promise<T | undefined> {
  try {
    return await (await db()).get("assets", key);
  } catch {
    return undefined;
  }
}
export async function cachePut(key: string, value: unknown) {
  try {
    const store = await db();
    const tx = store.transaction("assets", "readwrite");
    if ((await tx.store.count()) >= 100) {
      const oldest = await tx.store.openCursor();
      await oldest?.delete();
    }
    await tx.store.put(value, key);
    await tx.done;
  } catch {
    /* Cache failure must not discard a successful geographic response. */
  }
}

export async function saveNamed(s: Scenario) {
  const store = await db();
  if (
    !(await store.get("scenarios", `named:${s.name}`)) &&
    (await store.count("scenarios")) >= 50
  )
    throw Error("Scenario library limit: 50 entries");
  await store.put("scenarios", s, `named:${s.name}`);
}
export async function listNamed(): Promise<Scenario[]> {
  return (
    await (
      await db()
    ).getAll("scenarios", IDBKeyRange.bound("named:", "named:\uffff"))
  ).filter((s): s is Scenario => s && typeof s.name === "string");
}
