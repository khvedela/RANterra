// Deployment configuration is public. Never put private API keys here.
export const config = {
  tiles: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
  attribution: "© OpenStreetMap contributors · ODbL",
  search: "",
  overpass: "https://overpass-api.de/api/interpreter",
  osmMap: "",
};
export function validateConfig(data: unknown) {
  if (!data || typeof data !== "object" || Array.isArray(data))
    throw Error("Invalid deployment config");
  if (Object.keys(data).some((key) => !Object.hasOwn(config, key)))
    throw Error("Unknown deployment config field");
  const candidate = { ...config };
  for (const key of Object.keys(config) as (keyof typeof config)[]) {
    const value = (data as Record<string, unknown>)[key];
    if (value === undefined) continue;
    if (typeof value !== "string" || value.length > 2000)
      throw Error("Invalid deployment config field");
    if (
      key !== "attribution" &&
      value &&
      !/^https:\/\//.test(value) &&
      !(
        value.startsWith("/") &&
        !value.startsWith("//") &&
        !value.includes("\\")
      )
    )
      throw Error("Provider must use HTTPS or an origin-relative URL");
    if (key !== "attribution" && value) {
      const url = new URL(value, location.origin);
      if (url.username || url.password || value.includes("\\"))
        throw Error("Provider URL must not contain credentials or backslashes");
    }
    if (key === "attribution" && /[<>]/.test(value))
      throw Error("Attribution must be plain text");
    candidate[key] = value;
  }
  return candidate;
}
export async function loadConfig() {
  try {
    const r = await fetch(`${import.meta.env.BASE_URL}config.json`, {
      signal: AbortSignal.timeout(5000),
    });
    if (!r.ok) return;
    const data: unknown = JSON.parse(await boundedText(r, 16000));
    Object.assign(config, validateConfig(data));
  } catch (e) {
    console.warn("Using default geographic configuration", e);
  }
}
export async function boundedText(
  response: Response,
  limit: number,
): Promise<string> {
  const reader = response.body?.getReader();
  if (!reader) throw Error("Empty response");
  const decoder = new TextDecoder();
  let size = 0,
    text = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > limit) {
        await reader.cancel();
        throw Error("Geographic response exceeds size limit");
      }
      text += decoder.decode(value, { stream: true });
    }
  } finally {
    reader.releaseLock();
  }
  return text + decoder.decode();
}
