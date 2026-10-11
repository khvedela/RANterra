import { openDB } from "idb";
import type { Scenario, Recording } from "./types";
let connection: ReturnType<typeof openDB> | undefined;
const db = () =>
  (connection ??= openDB("ranterra", 2, {
    blocking() {
      const old = connection;
      connection = undefined;
      void old?.then((db) => db.close()).catch(() => undefined);
    },
    terminated() {
      connection = undefined;
    },
    upgrade(db) {
      for (const name of ["scenarios", "assets", "runs"])
        if (!db.objectStoreNames.contains(name)) db.createObjectStore(name);
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
  const tx = store.transaction("scenarios", "readwrite");
  const key = `named:${s.name}`;
  if (!(await tx.store.get(key)) && (await tx.store.count()) >= 50) {
    tx.abort();
    await tx.done.catch(() => undefined);
    throw Error("Scenario library limit: 50 entries");
  }
  await tx.store.put(s, key);
  await tx.done;
}
export async function listNamed(): Promise<Scenario[]> {
  return (
    await (
      await db()
    ).getAll("scenarios", IDBKeyRange.bound("named:", "named:\uffff"))
  ).filter((s): s is Scenario => s && typeof s.name === "string");
}

export async function saveRun(recording: Recording) {
  await (await db()).put("runs", recording, "current");
}
export async function loadRun(): Promise<Recording | undefined> {
  return (await db()).get("runs", "current");
}
