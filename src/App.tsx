import { useEffect, useRef, useState } from "react";
import {
  Activity,
  ArrowDownToLine,
  ArrowUpFromLine,
  Antenna,
  Box,
  Check,
  ChevronRight,
  Copy,
  FlaskConical,
  Info,
  Layers,
  MapPin,
  MousePointer2,
  Move,
  Pause,
  Play,
  Plus,
  Radio,
  RotateCcw,
  Search,
  Settings2,
  ShieldAlert,
  Signal,
  SkipForward,
  Smartphone,
  Trash2,
  X,
} from "lucide-react";
import NetworkMap from "./NetworkMap";
import { demo } from "./defaults";
import { loadScenario, saveScenario } from "./storage";
import { fetchBuildings, searchPlaces, type Place } from "./geography";
import type {
  Building,
  Gnodeb,
  NetworkEvent,
  Point,
  Scenario,
  Snapshot,
  ToolMode,
  WorkerCommand,
  WorkerResponse,
} from "./types";
function download(name: string, data: unknown) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
  );
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function prettyState(s?: string) {
  return s?.replace(/([a-z])([A-Z])/g, "$1 $2") ?? "Idle";
}
export default function App() {
  const [scenario, setScenario] = useState<Scenario>(demo),
    [snapshot, setSnapshot] = useState<Snapshot | null>(null),
    [running, setRunning] = useState(false),
    [pending, setPending] = useState(true),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [saved, setSaved] = useState(false);
  const [selected, setSelected] = useState<number | null>(1),
    [mode, setMode] = useState<ToolMode>("select"),
    [coverage, setCoverage] = useState(true),
    [buildings, setBuildings] = useState(true),
    [links, setLinks] = useState(true),
    [modelOpen, setModelOpen] = useState(false);
  const [query, setQuery] = useState(""),
    [places, setPlaces] = useState<Place[]>([]),
    [searching, setSearching] = useState(false),
    [loadingBuildings, setLoadingBuildings] = useState(false),
    [eventFilter, setEventFilter] = useState("all"),
    [eventId, setEventId] = useState<number | null>(null);
  const worker = useRef<Worker | null>(null),
    revision = useRef(0),
    activeRevision = useRef(0),
    candidate = useRef<Scenario | null>(null),
    scenarioRef = useRef(scenario),
    file = useRef<HTMLInputElement>(null);
  scenarioRef.current = scenario;
  const gnodeb = scenario.gnodebs.find((g) => g.id === selected),
    ue = scenario.ues.find((u) => u.id === selected),
    entity = gnodeb ?? ue;
  const ueStatus = snapshot?.ues.find((u) => u.id === selected);
  function command(type: "play" | "pause" | "step") {
    worker.current?.postMessage({
      type,
      revision: activeRevision.current,
    } satisfies WorkerCommand);
  }
  function apply(s: Scenario) {
    setPending(true);
    setRunning(false);
    setError("");
    setSaved(false);
    candidate.current = s;
    revision.current++;
    worker.current?.postMessage({
      type: "load",
      scenario: s,
      revision: revision.current,
    } satisfies WorkerCommand);
  }
  useEffect(() => {
    let cancelled = false;
    const w = new Worker(new URL("./engine.worker.ts", import.meta.url), {
      type: "module",
    });
    worker.current = w;
    w.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const data = e.data;
      if (data.revision !== revision.current) return;
      if (data.type === "error") {
        setError(`Rust/WASM engine: ${data.message}`);
        setPending(false);
        setRunning(false);
        candidate.current = null;
        revision.current = activeRevision.current;
        return;
      }
      activeRevision.current = data.revision;
      setSnapshot(data.snapshot);
      setRunning(data.running);
      setPending(false);
      if (candidate.current) {
        const accepted = candidate.current;
        candidate.current = null;
        setScenario(accepted);
        scenarioRef.current = accepted;
        setEventId(null);
        void saveScenario(accepted)
          .then(() => setSaved(true))
          .catch(() =>
            setNotice(
              "Browser storage is unavailable. Export JSON to keep your scenario.",
            ),
          );
      }
    };
    w.onerror = (e) => {
      setError(
        `WASM worker failed: ${e.message}. Rebuild WASM and reload; no fallback engine is used.`,
      );
      setRunning(false);
      setPending(false);
    };
    void (async () => {
      let initial = structuredClone(demo);
      try {
        const stored = await loadScenario();
        if (stored) initial = stored;
        else {
          const r = await fetch("/paris-buildings.json");
          if (r.ok) initial.buildings = (await r.json()) as Building[];
        }
      } catch {
        setNotice(
          "Saved scenario could not be loaded. Opening the built-in demo.",
        );
      }
      if (!cancelled) apply(initial);
    })();
    return () => {
      cancelled = true;
      w.terminate();
    };
  }, []);
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(""), 7000);
    return () => clearTimeout(t);
  }, [notice]);
  function move(id: number, p: Point) {
    apply({
      ...scenario,
      gnodebs: scenario.gnodebs.map((g) =>
        g.id === id ? { ...g, lon: p[0], lat: p[1] } : g,
      ),
      ues: scenario.ues.map((u) =>
        u.id === id ? { ...u, lon: p[0], lat: p[1] } : u,
      ),
    });
  }
  function place(p: Point) {
    if (pending) return;
    if (mode === "move" && selected) {
      move(selected, p);
      setMode("select");
      return;
    }
    if (mode !== "gnodeb" && mode !== "ue") return;
    if (
      (mode === "gnodeb" && scenario.gnodebs.length >= 16) ||
      (mode === "ue" && scenario.ues.length >= 128)
    ) {
      setNotice("Scenario entity limit reached.");
      return;
    }
    const id =
      Math.max(
        0,
        ...scenario.gnodebs.map((x) => x.id),
        ...scenario.ues.map((x) => x.id),
      ) + 1;
    if (mode === "gnodeb")
      apply({
        ...scenario,
        gnodebs: [
          ...scenario.gnodebs,
          {
            id,
            name: `gNodeB ${id}`,
            lon: p[0],
            lat: p[1],
            frequency_mhz: 3500,
            tx_power_dbm: 46,
            height_m: 25,
            gain_dbi: 16,
          },
        ],
      });
    else
      apply({
        ...scenario,
        ues: [...scenario.ues, { id, name: `UE-${id}`, lon: p[0], lat: p[1] }],
      });
    setSelected(id);
    setMode("select");
  }
  function remove() {
    apply({
      ...scenario,
      gnodebs: scenario.gnodebs.filter((g) => g.id !== selected),
      ues: scenario.ues.filter((u) => u.id !== selected),
    });
    setSelected(null);
  }
  function updateRadio(key: keyof Gnodeb, value: number) {
    if (!gnodeb) return;
    apply({
      ...scenario,
      gnodebs: scenario.gnodebs.map((g) =>
        g.id === gnodeb.id ? { ...g, [key]: value } : g,
      ),
    });
  }
  async function search(e: React.FormEvent) {
    e.preventDefault();
    if (!query.trim()) return;
    setSearching(true);
    setPlaces([]);
    const coordinates = query.split(",").map(Number);
    if (
      coordinates.length === 2 &&
      coordinates.every(Number.isFinite) &&
      Math.abs(coordinates[0]) <= 180 &&
      Math.abs(coordinates[1]) <= 85
    ) {
      apply({ ...scenario, center: coordinates as Point });
      setSearching(false);
      setNotice(
        "Study center changed. Existing entities retain their coordinates.",
      );
      return;
    }
    try {
      const results = await searchPlaces(query);
      setPlaces(results);
      if (!results.length)
        setNotice("No locations found. Try longitude, latitude.");
    } catch (e) {
      setNotice(String(e));
    } finally {
      setSearching(false);
    }
  }
  async function loadBuildings() {
    const base = scenario;
    setLoadingBuildings(true);
    try {
      const result = await fetchBuildings(base.center);
      if (scenarioRef.current !== base) {
        setNotice(
          "Scenario changed while loading. Click Load OSM to try again.",
        );
        return;
      }
      apply({ ...base, buildings: result });
      setNotice(
        `${result.length} OSM building footprints loaded. Only simple, closed ways are used; dataset may be partial.`,
      );
    } catch (e) {
      setNotice(String(e));
    } finally {
      setLoadingBuildings(false);
    }
  }
  async function importFile(e: React.ChangeEvent<HTMLInputElement>) {
    const f = e.target.files?.[0];
    if (!f) return;
    try {
      if (f.size > 2_000_000)
        throw Error("Scenario must be smaller than 2 MB.");
      const data: unknown = JSON.parse(await f.text());
      if (!data || typeof data !== "object")
        throw Error("Expected a scenario object.");
      apply(data as Scenario);
      setSelected(null);
    } catch (e) {
      setNotice(`Import failed: ${e}`);
    } finally {
      e.target.value = "";
    }
  }
  const events = (snapshot?.events ?? []).filter(
    (e) => eventFilter === "all" || e.layer === eventFilter,
  );
  const selectedEvent = events.find((e) => e.id === eventId) ?? events.at(-1);
  const completed = snapshot?.started && snapshot.pending === 0;
  const mean = snapshot?.ues
    .filter((u) => u.rsrp !== null)
    .reduce((a, u) => a + u.rsrp!, 0);
  const signalCount = snapshot?.ues.filter((u) => u.rsrp !== null).length ?? 0;
  return (
    <div className="app-shell">
      <header className="topbar">
        <a href="/" className="brand">
          <span className="brand-symbol">
            <Layers size={23} />
          </span>
          RAN<span>terra</span>
          <small>RESEARCH PLAYGROUND</small>
        </a>
        <div className="top-tabs">
          <span className="active">
            <Box size={15} /> Workspace
          </span>
          <button onClick={() => setModelOpen(true)}>
            <FlaskConical size={15} /> Models & assumptions
          </button>
        </div>
        <div className="runtime">
          <span className={`live-dot ${error ? "bad" : ""}`} />
          {error
            ? "Engine error"
            : pending
              ? "Loading engine"
              : "Rust + WebAssembly"}
          <span className="local-pill">LOCAL</span>
        </div>
      </header>
      <div className="workspace-bar">
        <div>
          <div className="eyebrow">
            EXPERIMENT / 001 <span>5G SA · CONTROL PLANE</span>
          </div>
          <h1>
            <input
              aria-label="Scenario name"
              key={scenario.name}
              defaultValue={scenario.name}
              maxLength={120}
              onBlur={(e) => {
                if (e.target.value.trim() && e.target.value !== scenario.name)
                  apply({ ...scenario, name: e.target.value.trim() });
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
              }}
            />
          </h1>
        </div>
        <div className="workspace-actions">
          <span className="saved">
            <Check size={13} />
            {saved
              ? "Saved on this device"
              : pending
                ? "Computing…"
                : "Local workspace"}
          </span>
          <button onClick={() => file.current?.click()}>
            <ArrowUpFromLine size={15} /> Import
          </button>
          <button onClick={() => download("ranterra-scenario.json", scenario)}>
            <ArrowDownToLine size={15} /> Export
          </button>
          <button
            className="run-button"
            disabled={
              pending || !!error || scenario.ues.length === 0 || !!completed
            }
            onClick={() => command(running ? "pause" : "play")}
          >
            {running ? <Pause size={15} /> : <Play size={15} />}{" "}
            {running
              ? "Pause"
              : completed
                ? "Exchange complete"
                : snapshot?.started
                  ? "Resume"
                  : "Run simulation"}
          </button>
        </div>
      </div>
      <main className="workspace">
        <aside className="left-panel">
          <section className="location-section">
            <div className="section-label">
              <span>STUDY AREA</span>
              <MapPin size={14} />
            </div>
            <form onSubmit={search} className="search-box">
              <Search size={16} />
              <input
                aria-label="Search location"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search city or lon, lat"
              />
              <button
                aria-label="Find location"
                disabled={searching}
                type="submit"
              >
                {searching ? "…" : "↵"}
              </button>
            </form>
            {places.length > 0 && (
              <div className="place-results">
                {places.map((p, i) => (
                  <button
                    key={i}
                    onClick={() => {
                      apply({
                        ...scenario,
                        center: [Number(p.lon), Number(p.lat)],
                      });
                      setPlaces([]);
                      setQuery("");
                      setNotice(
                        "Study center changed. Entities retain their coordinates; move or add them in the new area.",
                      );
                    }}
                  >
                    {p.display_name}
                  </button>
                ))}
              </div>
            )}
            <div className="area-meta">
              <span>Center</span>
              <span>
                {scenario.center[1].toFixed(4)}, {scenario.center[0].toFixed(4)}
              </span>
            </div>
            <button
              className="load-buildings"
              disabled={loadingBuildings || pending}
              onClick={loadBuildings}
            >
              <Box size={14} />
              {loadingBuildings ? "Loading geometry…" : "Load OSM buildings"}
              <span>{scenario.buildings.length}</span>
            </button>
          </section>
          <section>
            <div className="section-label">
              <span>DEPLOY NETWORK</span>
              <Plus size={14} />
            </div>
            <div className="deploy-buttons">
              <button
                className={mode === "gnodeb" ? "chosen" : ""}
                onClick={() => setMode(mode === "gnodeb" ? "select" : "gnodeb")}
                disabled={pending}
              >
                <Antenna size={21} />
                <strong>gNodeB</strong>
                <span>Base station</span>
              </button>
              <button
                className={mode === "ue" ? "chosen" : ""}
                onClick={() => setMode(mode === "ue" ? "select" : "ue")}
                disabled={pending}
              >
                <Smartphone size={21} />
                <strong>UE</strong>
                <span>User equipment</span>
              </button>
            </div>
            <p className="hint">Choose an entity, then place it on the map.</p>
          </section>
          <section className="entities-section">
            <div className="section-label">
              <span>NETWORK ENTITIES</span>
              <span className="count">
                {scenario.gnodebs.length + scenario.ues.length}
              </span>
            </div>
            <div className="entity-group-label">
              gNodeBs <span>{scenario.gnodebs.length}</span>
            </div>
            {scenario.gnodebs.map((g, i) => (
              <button
                key={g.id}
                className={`entity-row ${selected === g.id ? "selected" : ""}`}
                onClick={() => {
                  setSelected(g.id);
                  setMode("select");
                }}
              >
                <span className="entity-icon">
                  <Antenna size={17} />
                </span>
                <span>
                  <strong>{g.name}</strong>
                  <small>
                    NR lab · {(g.frequency_mhz / 1000).toFixed(2)} GHz
                  </small>
                </span>
                <span className="cell-code">
                  {String(i + 1).padStart(2, "0")}
                </span>
              </button>
            ))}
            <div className="entity-group-label ue-group">
              User equipment <span>{scenario.ues.length}</span>
            </div>
            {scenario.ues.map((u) => {
              const status = snapshot?.ues.find((x) => x.id === u.id);
              return (
                <button
                  key={u.id}
                  className={`entity-row ue-row ${selected === u.id ? "selected" : ""}`}
                  onClick={() => {
                    setSelected(u.id);
                    setMode("select");
                  }}
                >
                  <Smartphone size={16} />
                  <strong>{u.name}</strong>
                  <span
                    className={`signal-value ${(status?.rsrp ?? -140) < -110 ? "weak" : ""}`}
                  >
                    {status?.rsrp != null ? `${status.rsrp.toFixed(0)}` : "—"}
                    <small> dBm</small>
                  </span>
                </button>
              );
            })}
          </section>
          <section className="layer-section">
            <div className="section-label">
              <span>MAP LAYERS</span>
              <Layers size={14} />
            </div>
            {[
              ["Coverage estimate", coverage, setCoverage],
              ["Building footprints", buildings, setBuildings],
              ["Serving-cell links", links, setLinks],
            ].map(([label, value, setter]) => (
              <label className="toggle-row" key={label as string}>
                <span>{label as string}</span>
                <input
                  type="checkbox"
                  checked={value as boolean}
                  onChange={(e) =>
                    (setter as React.Dispatch<React.SetStateAction<boolean>>)(
                      e.target.checked,
                    )
                  }
                />
                <span className="toggle" />
              </label>
            ))}
          </section>
          <div className="sidebar-footer">
            <FlaskConical size={16} />
            <span>
              Built for exploration.
              <br />
              <strong>Every model has boundaries.</strong>
            </span>
            <button
              title="Read model limitations"
              onClick={() => setModelOpen(true)}
            >
              <Info size={16} />
            </button>
          </div>
        </aside>
        <div className="center-panel">
          <div className="map-header">
            <div>
              <span className="tab-active">
                <MapPin size={14} /> Network map
              </span>
              <span className="map-subtitle">1.7 × 1.7 km study grid</span>
            </div>
            <div>
              <button
                className={mode === "select" ? "active" : ""}
                title="Select and drag entities"
                onClick={() => setMode("select")}
              >
                <MousePointer2 size={15} />
              </button>
              <button
                className={mode === "move" ? "active" : ""}
                title="Move selected entity"
                disabled={!entity}
                onClick={() => setMode("move")}
              >
                <Move size={15} />
              </button>
            </div>
          </div>
          <NetworkMap
            scenario={scenario}
            snapshot={snapshot}
            selected={selected}
            mode={mode}
            coverage={coverage}
            buildings={buildings}
            links={links}
            onSelect={(id) => {
              setSelected(id);
              setMode("select");
            }}
            onPlace={place}
            onMove={move}
            onCenter={(p) => apply({ ...scenario, center: p })}
          />
          <div className="simulation-strip">
            <span className={`sim-status ${running ? "running" : ""}`}>
              <span className="live-dot" />
              {running
                ? "RUNNING"
                : completed
                  ? "COMPLETE"
                  : snapshot?.started
                    ? "PAUSED"
                    : "READY"}
            </span>
            <div className="playback">
              <button
                title="Reset simulation"
                aria-label="Reset simulation"
                disabled={pending}
                onClick={() => apply({ ...scenario })}
              >
                <RotateCcw size={15} />
              </button>
              <button
                title="Advance 10 milliseconds"
                aria-label="Step simulation"
                disabled={pending || !!error || !!completed}
                onClick={() => command("step")}
              >
                <SkipForward size={15} />
              </button>
              <span className="sim-clock">
                {((snapshot?.time_ms ?? 0) / 1000).toFixed(3)}
                <small> s</small>
              </span>
            </div>
            <label className="seed">
              SEED{" "}
              <input
                key={scenario.seed}
                aria-label="Random seed"
                type="number"
                min="0"
                max="4294967295"
                defaultValue={scenario.seed}
                onBlur={(e) => {
                  const n = Number(e.target.value);
                  if (
                    Number.isInteger(n) &&
                    n >= 0 &&
                    n <= 4294967295 &&
                    n !== scenario.seed
                  )
                    apply({ ...scenario, seed: n });
                }}
              />
            </label>
            <span className="speed">0.1× playback</span>
          </div>
          <section className="timeline">
            <div className="timeline-heading">
              <h2>
                <Activity size={16} /> Protocol timeline{" "}
                <span>{snapshot?.events.length ?? 0}</span>
              </h2>
              <div className="timeline-filters">
                {["all", "NAS", "NGAP", "RADIO"].map((f) => (
                  <button
                    className={eventFilter === f ? "active" : ""}
                    key={f}
                    onClick={() => setEventFilter(f)}
                  >
                    {f === "all" ? "All events" : f}
                  </button>
                ))}
                <button
                  title="Export event log"
                  aria-label="Export event log"
                  onClick={() =>
                    download("ranterra-events.json", snapshot?.events ?? [])
                  }
                >
                  <ArrowDownToLine size={14} />
                </button>
              </div>
            </div>
            <div className="timeline-body">
              <div className="event-list">
                {!events.length ? (
                  <div className="empty-events">
                    <span>
                      <Radio size={23} />
                    </span>
                    <strong>Your network is ready to talk.</strong>
                    <p>
                      Run the simulation to inspect real NAS and NGAP messages.
                    </p>
                    <small>UE → gNodeB → AMF → gNodeB → UE</small>
                  </div>
                ) : (
                  events.map((e) => (
                    <button
                      className={`event-row ${selectedEvent?.id === e.id ? "active" : ""}`}
                      key={e.id}
                      onClick={() => setEventId(e.id)}
                    >
                      <time>{(e.time_ms / 1000).toFixed(3)}</time>
                      <span
                        className={`protocol-badge ${e.layer.toLowerCase()}`}
                      >
                        {e.layer}
                      </span>
                      <span>
                        <strong>{e.kind}</strong>
                        <small>
                          {e.source} → {e.destination}
                        </small>
                      </span>
                      <ChevronRight size={13} />
                    </button>
                  ))
                )}
              </div>
              {selectedEvent && <EventDetail event={selectedEvent} />}
            </div>
          </section>
        </div>
        <aside className="right-panel">
          <div className="inspector-title">
            <span>
              <Settings2 size={16} /> Entity inspector
            </span>
            <span className="tiny-tag">LIVE</span>
          </div>
          {entity ? (
            <>
              <div className="inspector-identity">
                <div className="large-entity-icon">
                  {gnodeb ? <Antenna size={25} /> : <Smartphone size={25} />}
                </div>
                <div>
                  <span className="eyebrow">
                    {gnodeb ? "NEXT GENERATION NODE B" : "USER EQUIPMENT"}
                  </span>
                  <h2>{entity.name}</h2>
                  <span className="entity-state">
                    <span className="live-dot" />
                    {gnodeb
                      ? (snapshot?.gnodebs[entity.id] ?? "Ready")
                      : prettyState(ueStatus?.state)}
                  </span>
                </div>
              </div>
              <section className="inspector-section">
                <div className="section-label">
                  {gnodeb ? "RADIO CONFIGURATION" : "RADIO MEASUREMENT"}
                </div>
                {gnodeb ? (
                  <>
                    <RadioField
                      key={`${gnodeb.id}-f-${gnodeb.frequency_mhz}`}
                      label="Carrier frequency"
                      unit="MHz"
                      value={gnodeb.frequency_mhz}
                      min={100}
                      max={100000}
                      step={100}
                      onChange={(v) => updateRadio("frequency_mhz", v)}
                    />
                    <RadioField
                      key={`${gnodeb.id}-p-${gnodeb.tx_power_dbm}`}
                      label="Transmit power"
                      unit="dBm"
                      value={gnodeb.tx_power_dbm}
                      min={-20}
                      max={80}
                      onChange={(v) => updateRadio("tx_power_dbm", v)}
                    />
                    <RadioField
                      key={`${gnodeb.id}-h-${gnodeb.height_m}`}
                      label="Antenna height"
                      unit="m"
                      value={gnodeb.height_m}
                      min={1.5}
                      max={300}
                      step={0.5}
                      onChange={(v) => updateRadio("height_m", v)}
                    />
                    <RadioField
                      key={`${gnodeb.id}-g-${gnodeb.gain_dbi}`}
                      label="Antenna gain"
                      unit="dBi"
                      value={gnodeb.gain_dbi}
                      min={-10}
                      max={40}
                      onChange={(v) => updateRadio("gain_dbi", v)}
                    />
                    <div className="radio-footnote">
                      <Info size={13} />
                      <span>
                        Omnidirectional approximation. Fixed 106 RB power
                        allocation.
                      </span>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="signal-readout">
                      <Signal size={22} />
                      <strong>{ueStatus?.rsrp?.toFixed(1) ?? "—"}</strong>
                      <span>dBm RSRP</span>
                    </div>
                    <div className="detail-row">
                      <span>Serving cell</span>
                      <strong>
                        {ueStatus?.serving_cell
                          ? `gNB-${ueStatus.serving_cell}`
                          : "No coverage"}
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>Registration</span>
                      <strong>Not registered</strong>
                    </div>
                    <div className="detail-row">
                      <span>AMF context</span>
                      <strong>
                        {snapshot?.amf[entity.id] ?? "Not created"}
                      </strong>
                    </div>
                    <p className="hint">
                      Cell selection uses strongest estimated RSRP above −120
                      dBm. No handover procedure is simulated.
                    </p>
                  </>
                )}
              </section>
              <section className="inspector-section">
                <div className="section-label">
                  POSITION <MapPin size={13} />
                </div>
                <label className="entity-name-field">
                  Entity name
                  <input
                    aria-label="Entity name"
                    key={`${entity.id}-${entity.name}`}
                    defaultValue={entity.name}
                    maxLength={80}
                    onBlur={(e) => {
                      const name = e.target.value.trim();
                      if (name && name !== entity.name)
                        apply({
                          ...scenario,
                          gnodebs: scenario.gnodebs.map((g) =>
                            g.id === selected ? { ...g, name } : g,
                          ),
                          ues: scenario.ues.map((u) =>
                            u.id === selected ? { ...u, name } : u,
                          ),
                        });
                    }}
                  />
                </label>
                <div className="coordinate-fields">
                  <label>
                    Latitude
                    <input
                      key={`${entity.id}-lat-${entity.lat}`}
                      type="number"
                      step="0.00001"
                      min="-85"
                      max="85"
                      defaultValue={entity.lat.toFixed(5)}
                      onBlur={(e) => {
                        if (
                          e.target.checkValidity() &&
                          e.target.value &&
                          Number(e.target.value) !==
                            Number(entity.lat.toFixed(5))
                        )
                          move(entity.id, [entity.lon, Number(e.target.value)]);
                      }}
                    />
                  </label>
                  <label>
                    Longitude
                    <input
                      key={`${entity.id}-lon-${entity.lon}`}
                      type="number"
                      step="0.00001"
                      min="-180"
                      max="180"
                      defaultValue={entity.lon.toFixed(5)}
                      onBlur={(e) => {
                        if (
                          e.target.checkValidity() &&
                          e.target.value &&
                          Number(e.target.value) !==
                            Number(entity.lon.toFixed(5))
                        )
                          move(entity.id, [Number(e.target.value), entity.lat]);
                      }}
                    />
                  </label>
                </div>
                <button
                  className="full-button"
                  onClick={() => setMode(mode === "move" ? "select" : "move")}
                >
                  <Move size={14} />
                  {mode === "move"
                    ? "Click destination on map"
                    : "Reposition on map"}
                </button>
              </section>
              <section className="inspector-section">
                <div className="section-label">
                  {gnodeb ? "CELL OVERVIEW" : "EXPERIMENT STATUS"}
                </div>
                {gnodeb ? (
                  <>
                    <div className="detail-row">
                      <span>Associated UEs</span>
                      <strong>
                        {snapshot?.ues.filter(
                          (u) => u.serving_cell === gnodeb.id,
                        ).length ?? 0}
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>Registered UEs</span>
                      <strong>0</strong>
                    </div>
                    <div className="detail-row">
                      <span>Core network</span>
                      <strong>
                        <span className="live-dot" /> Virtual AMF
                      </strong>
                    </div>
                  </>
                ) : (
                  <div className="protocol-note">
                    <ShieldAlert size={17} />
                    <span>
                      Registration is deliberately rejected. Authentication and
                      security are not implemented.
                    </span>
                  </div>
                )}
              </section>
              <button
                className="delete-entity"
                onClick={remove}
                disabled={pending}
              >
                <Trash2 size={14} /> Delete {gnodeb ? "gNodeB" : "UE"}
              </button>
            </>
          ) : (
            <div className="inspector-empty">
              <MousePointer2 size={28} />
              <h2>Inspect your network</h2>
              <p>Select a base station or UE on the map.</p>
            </div>
          )}
          <div className="network-summary">
            <span className="section-label">NETWORK SNAPSHOT</span>
            <div>
              <span>
                <strong>{scenario.gnodebs.length}</strong>cells
              </span>
              <span>
                <strong>{scenario.ues.length}</strong>UEs
              </span>
              <span>
                <strong>
                  {signalCount && mean !== undefined
                    ? Math.round(mean / signalCount)
                    : "—"}
                </strong>
                avg. dBm
              </span>
            </div>
          </div>
        </aside>
      </main>
      <footer className="statusbar">
        <span>
          <span className="live-dot" /> Browser-native simulation{" "}
          <span className="muted">/</span> No application backend
        </span>
        <span>
          OxiRush NAS + NGAP <span className="muted">/</span> Approximate RF{" "}
          <span className="muted">/</span>
          <button onClick={() => setModelOpen(true)}>
            MVP v0.1 <Info size={11} />
          </button>
        </span>
      </footer>
      {error && (
        <div className="error-banner" role="alert">
          <ShieldAlert size={20} />
          <span>{error}</span>
          <button onClick={() => apply(structuredClone(demo))}>
            Reset to demo
          </button>
        </div>
      )}
      {notice && (
        <div className="toast" role="status">
          <Info size={17} />
          <span>{notice}</span>
          <button title="Dismiss notification" onClick={() => setNotice("")}>
            <X size={15} />
          </button>
        </div>
      )}
      {modelOpen && (
        <div className="modal-scrim" onClick={() => setModelOpen(false)}>
          <div
            className="model-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Models and assumptions"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              className="modal-close"
              aria-label="Close model information"
              onClick={() => setModelOpen(false)}
            >
              <X size={20} />
            </button>
            <span className="eyebrow">KNOW YOUR MODEL</span>
            <h2>A playground, with clear boundaries.</h2>
            <p>
              RANterra executes a deterministic Rust engine in a Web Worker. NAS
              and NGAP messages are encoded and decoded by OxiRush, compiled to
              WebAssembly.
            </p>
            <h3>Radio propagation</h3>
            <code>
              PL = 32.4 + 20 log₁₀(f GHz) + 30 log₁₀(d metres)
              <br />
              RSRP ≈ Ptx + gain − 10 log₁₀(12 × 106) − PL − Lbuildings
            </code>
            <p>
              Close-in log-distance model, exponent 3.0; 3D distance with UE
              height 1.5 m. Fixed 106 resource blocks; no interference, fading,
              terrain, beamforming, scheduling or calibrated antenna pattern.
              Frequencies outside sub-6 GHz are experimental extrapolations.
            </p>
            <h3>Buildings are a coarse proxy</h3>
            <p>
              Each intersected OSM footprint adds 12 dB, capped at 36 dB. Only
              closed simple ways are loaded, up to 600 buildings. Heights,
              materials, holes and multipolygon relations are not modeled.
              Missing buildings mean no obstruction loss.
            </p>
            <h3>Real messages. Deliberately incomplete registration.</h3>
            <p>
              UE → NAS Registration Request → NGAP Initial UE Message → AMF →
              Registration Reject (cause 7) → NGAP Downlink NAS Transport → UE.
              The AMF uses a lab reject policy. No authentication, NAS security,
              Registration Accept, PDU session, RRC, NG setup, SCTP or user
              plane is implemented.
            </p>
            <h3>Local and reproducible</h3>
            <p>
              The seed controls virtual link delays (10–30 ms per hop). Editing
              the scenario resets the experiment. Search, tiles and new OSM
              geometry need internet; loaded geometry, protocol execution,
              coverage and JSON exports work locally. 6G radio models and AI
              controllers are extension points, not implemented features.
            </p>
            <button className="run-button" onClick={() => setModelOpen(false)}>
              Back to the experiment <ChevronRight size={15} />
            </button>
          </div>
        </div>
      )}
      <input
        ref={file}
        type="file"
        accept=".json,application/json"
        hidden
        onChange={importFile}
      />
    </div>
  );
}
function RadioField({
  label,
  unit,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  unit: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  return (
    <label className="radio-field">
      <span>{label}</span>
      <div>
        <input
          aria-label={label}
          type="number"
          min={min}
          max={max}
          step={step}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={(e) => {
            if (
              e.target.checkValidity() &&
              draft !== "" &&
              Number(draft) !== value
            )
              onChange(Number(draft));
            else setDraft(String(value));
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
        <span>{unit}</span>
      </div>
    </label>
  );
}
function EventDetail({ event }: { event: NetworkEvent }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="event-detail">
      <div className="event-detail-heading">
        <span>DECODED PAYLOAD</span>
        <button
          title="Copy message bytes"
          onClick={() =>
            void navigator.clipboard.writeText(event.hex).then(() => {
              setCopied(true);
              setTimeout(() => setCopied(false), 1200);
            })
          }
        >
          {copied ? <Check size={13} /> : <Copy size={13} />}
        </button>
      </div>
      <strong>{event.kind}</strong>
      <p>{event.detail}</p>
      {event.hex && (
        <>
          <span className="hex-label">
            {event.layer} · {event.hex.split(" ").length} BYTES
          </span>
          <code>{event.hex}</code>
        </>
      )}
    </div>
  );
}
