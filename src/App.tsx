import { useEffect, useRef, useState, type CSSProperties } from "react";
import {
  Alert,
  Badge,
  Card,
  Checkbox,
  Empty,
  Input,
  Layout,
  Menu,
  Statistic,
  Switch,
  Table,
  Tag,
  theme,
} from "antd";
import {
  ControlButton,
  SelectField,
  WorkspacePanel,
  ResearchModal,
} from "./ui";
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
  RotateCcw,
  Search,
  Settings2,
  ShieldAlert,
  Signal,
  SkipForward,
  Smartphone,
  Trash2,
} from "lucide-react";
import NetworkMap from "./NetworkMap";
import { demo } from "./defaults";
import {
  loadScenario,
  saveScenario,
  saveNamed,
  listNamed,
  saveRun,
  loadRun,
} from "./storage";
import { fetchBuildings, searchPlaces, type Place } from "./geography";
import type {
  Building,
  Gnodeb,
  NetworkEvent,
  Point,
  Scenario,
  Recording,
  SessionPolicy,
  SessionRequest,
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
  const { token } = theme.useToken();
  const [setupOpen, setSetupOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [workerEpoch, setWorkerEpoch] = useState(0);
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
  const session = snapshot?.sessions.find((s) => s.ue_id === selected);
  const [speed, setSpeed] = useState(0.1);
  const speedRef = useRef(speed);
  speedRef.current = speed;
  const [library, setLibrary] = useState<Scenario[]>([]);
  const [policiesOpen, setPoliciesOpen] = useState(false);
  const [lossPercent, setLossPercent] = useState(0);
  const importPurpose = useRef<"restore" | "compare">("restore");
  const [comparison, setComparison] = useState<{
    name: string;
    snapshot: Snapshot;
  } | null>(null);
  const defaultRequest: SessionRequest = {
    dnn: "lab",
    snssai: { sst: 1, sd: null },
  };
  const policies = scenario.session_policies ?? [
    { request: defaultRequest, max_sessions: 128 },
  ];
  const ueRequest = ue?.session ?? defaultRequest;
  const signaling = (candidate.current ?? scenario).ues.find(
    (u) => u.id === selected,
  )?.signaling;
  const sessionFailure = selected
    ? snapshot?.session_failures?.[selected]
    : undefined;
  const [resultsOpen, setResultsOpen] = useState(false);
  const [baseline, setBaseline] = useState<{
    registered: number;
    latency: number | null;
  } | null>(null);
  const [ueFilter, setUeFilter] = useState("all");
  const radio = scenario.radio ?? {
    model: "Legacy" as const,
    shadow_fading: false,
    coverage_dbm: -120,
    hysteresis_db: 3,
    time_to_trigger_ms: 320,
    duration_ms: 10000,
  };
  const registered =
    snapshot?.ues.filter((u) => u.state === "Registered").length ?? 0;
  const failed =
    snapshot?.ues.filter((u) => u.state === "RegistrationRejected").length ?? 0;
  const latencies =
    snapshot?.ues.flatMap((u) =>
      u.registration_latency_ms === null ? [] : [u.registration_latency_ms],
    ) ?? [];
  const latency = latencies.length
    ? latencies.reduce((a, b) => a + b, 0) / latencies.length
    : null;
  function metricsCSV() {
    const rows = [
      "ue_id,state,cell_id,rsrp_dbm,sinr_db,registration_latency_virtual_ms",
      ...(snapshot?.ues ?? []).map((u) =>
        [
          u.id,
          u.state,
          u.serving_cell,
          u.rsrp,
          u.sinr_db,
          u.registration_latency_ms,
        ].join(","),
      ),
    ];
    const url = URL.createObjectURL(
      new Blob([rows.join("\n")], { type: "text/csv" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "ranterra-metrics.csv";
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function saveCopy() {
    try {
      const copy = { ...scenario, name: scenario.name.slice(0, 110) + " copy" };
      await saveNamed(copy);
      setLibrary(await listNamed());
      setNotice("Scenario copy saved in the local library.");
    } catch (e) {
      setNotice(String(e));
    }
  }

  function sessionAction(
    action: "establish" | "release" | "echo" | "deregister",
  ) {
    if (selected)
      worker.current?.postMessage({
        type: "session",
        id: selected,
        action,
        loss: lossPercent,
        revision: activeRevision.current,
      } satisfies WorkerCommand);
  }
  const sessionPending =
    snapshot?.session_pending?.includes(selected ?? 0) ?? false;
  const ueStatus = snapshot?.ues.find((u) => u.id === selected);
  function command(type: "play" | "pause" | "step") {
    worker.current?.postMessage({
      type,
      revision: activeRevision.current,
    } satisfies WorkerCommand);
  }
  async function restoreRun() {
    // Loading IndexedDB is asynchronous: disable export before reading so an old
    // revision cannot be exported while a checkpoint begins reconstruction.
    setPending(true);
    let applied = false;
    try {
      const recording = await loadRun();
      if (recording) {
        apply(recording);
        applied = true;
      } else setNotice("No saved run checkpoint.");
    } catch (e) {
      setNotice(String(e));
    } finally {
      if (!applied) setPending(false);
    }
  }
  function apply(s: Scenario | Recording) {
    setPending(true);
    setRunning(false);
    setError("");
    setSaved(false);
    candidate.current = "format" in s ? s.scenario : s;
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
      if (data.type === "recording") {
        if (data.purpose === "export")
          download("ranterra-experiment.json", data.recording);
        else
          void saveRun(data.recording)
            .then(() =>
              setNotice("Experiment checkpoint saved with command journal."),
            )
            .catch((e) => setNotice(`Save run failed: ${e}`));
        return;
      }
      if (data.type === "comparison") {
        setComparison({ name: data.name, snapshot: data.snapshot });
        return;
      }
      if (data.type === "error") {
        setError(`Rust/WASM engine: ${data.message}`);
        setPending(false);
        setRunning(false);
        candidate.current = null;
        revision.current = activeRevision.current;
        return;
      }
      const sameRevision = activeRevision.current === data.revision;
      activeRevision.current = data.revision;
      if (!sameRevision)
        w.postMessage({
          type: "speed",
          speed: speedRef.current,
          revision: data.revision,
        } satisfies WorkerCommand);
      setSnapshot((prev) =>
        sameRevision && prev
          ? {
              ...data.snapshot,
              coverage: prev.coverage,
              ues:
                JSON.stringify(prev.ues) === JSON.stringify(data.snapshot.ues)
                  ? prev.ues
                  : data.snapshot.ues,
            }
          : data.snapshot,
      );
      setRunning(data.running);
      setPending(false);
      if (candidate.current) {
        const accepted = data.scenario ?? candidate.current;
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
        `WASM worker failed: ${e.message}. Use Recover worker to restart from the saved scenario.`,
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
  }, [workerEpoch]);
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
    if (mode === "trajectory" && selected) {
      apply({
        ...scenario,
        ues: scenario.ues.map((u) =>
          u.id === selected
            ? {
                ...u,
                mobility: {
                  speed_mps: u.mobility?.speed_mps || 1.4,
                  waypoints: [...(u.mobility?.waypoints ?? []), p],
                },
              }
            : u,
        ),
      });
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
            height_m: radio.model === "UMi" ? 10 : 25,
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
  function changeSignaling(
    patch: Partial<NonNullable<Scenario["ues"][number]["signaling"]>>,
  ) {
    if (!ue) return;
    const base = candidate.current ?? scenario;
    apply({
      ...base,
      ues: base.ues.map((u) =>
        u.id === ue.id
          ? {
              ...u,
              signaling: {
                delay_ms: 20,
                drop: "None",
                count: 0,
                reject_n2: false,
                ...u.signaling,
                ...patch,
              },
            }
          : u,
      ),
    });
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
    setPending(true);
    let applied = false;
    try {
      if (f.size > 4_000_000)
        throw Error("Import must be smaller than 4 MB (scenarios: 2 MB).");
      const data: unknown = JSON.parse(await f.text());
      if (!data || typeof data !== "object")
        throw Error("Expected a scenario object.");
      if (importPurpose.current === "compare") {
        if (!("format" in data))
          throw Error("Comparison requires an exported experiment run.");
        worker.current?.postMessage({
          type: "compare",
          recording: data as Recording,
          revision: activeRevision.current,
        } satisfies WorkerCommand);
      } else {
        apply(data as Scenario | Recording);
        applied = true;
      }
      setSelected(null);
    } catch (e) {
      setNotice(`Import failed: ${e}`);
    } finally {
      if (!applied) setPending(false);
      e.target.value = "";
      importPurpose.current = "restore";
    }
  }
  const events = (snapshot?.events ?? []).filter(
    (e) =>
      (eventFilter === "all" || e.layer === eventFilter) &&
      (ueFilter === "all" || e.ue_id === Number(ueFilter)),
  );
  const selectedEvent = events.find((e) => e.id === eventId) ?? events.at(-1);
  const completed = snapshot?.started && snapshot.pending === 0;
  const mean = snapshot?.ues
    .filter((u) => u.rsrp !== null)
    .reduce((a, u) => a + u.rsrp!, 0);
  const signalCount = snapshot?.ues.filter((u) => u.rsrp !== null).length ?? 0;
  return (
    <Layout
      className="app-shell"
      style={
        {
          "--surface": token.colorBgContainer,
          "--layout": token.colorBgLayout,
          "--fill": token.colorFillAlter,
          "--line": token.colorBorderSecondary,
          "--border": token.colorBorder,
          "--ink": token.colorText,
          "--muted": token.colorTextSecondary,
          "--primary": token.colorPrimary,
          "--selection": token.controlItemBgActive,
          "--success": token.colorSuccess,
          "--error": token.colorError,
          "--warning": token.colorWarning,
          "--radius": `${token.borderRadius}px`,
          "--shadow": token.boxShadowSecondary,
          "--shadow-raised": token.boxShadowTertiary,
        } as CSSProperties
      }
    >
      <Layout.Header className="topbar">
        <a href="/" className="brand" aria-label="RANterra home">
          <span className="brand-symbol">
            <Layers size={22} />
          </span>
          <span>RANterra</span>
          <Tag color="blue">RESEARCH</Tag>
        </a>
        <Menu
          mode="horizontal"
          selectedKeys={["workspace"]}
          className="workspace-menu"
          items={[
            { key: "workspace", icon: <Box size={16} />, label: "Workspace" },
            {
              key: "models",
              icon: <FlaskConical size={16} />,
              label: "Models & assumptions",
              onClick: () => setModelOpen(true),
            },
          ]}
        />
        <div className="runtime">
          <Badge
            status={error ? "error" : pending ? "processing" : "success"}
            text={
              error
                ? "Engine error"
                : pending
                  ? "Loading engine"
                  : "Rust + WebAssembly"
            }
          />
          <Tag>LOCAL</Tag>
        </div>
      </Layout.Header>
      <div className="workspace-bar">
        <div>
          <div className="eyebrow">
            RESEARCH WORKSPACE <span>5G standalone · browser simulation</span>
          </div>
          <h1>
            <Input
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
          <span className="saved">
            <Check size={13} />
            {saved
              ? "Saved on this device"
              : pending
                ? "Computing…"
                : "Local workspace"}
          </span>
        </div>
        <div className="workspace-actions">
          <ControlButton onClick={() => setResultsOpen(true)}>
            Results
          </ControlButton>
          <ControlButton onClick={() => setPoliciesOpen(true)}>
            Session policies
          </ControlButton>
          <ControlButton onClick={() => void saveCopy()}>
            Save copy
          </ControlButton>
          <SelectField
            aria-label="Saved scenarios"
            value=""
            onFocus={() => {
              void listNamed()
                .then(setLibrary)
                .catch((e) => setNotice(String(e)));
            }}
            onChange={(e) => {
              const saved = library[Number(e.target.value)];
              if (saved) apply(saved);
            }}
          >
            <option value="">Saved scenarios</option>
            {library.map((s, i) => (
              <option key={i} value={i}>
                {s.name}
              </option>
            ))}
          </SelectField>
          <ControlButton
            onClick={() => {
              importPurpose.current = "restore";
              file.current?.click();
            }}
          >
            <ArrowUpFromLine size={15} /> Import
          </ControlButton>
          <ControlButton
            onClick={() => download("ranterra-scenario.json", scenario)}
          >
            <ArrowDownToLine size={15} /> Export
          </ControlButton>
        </div>
        <ControlButton
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
        </ControlButton>
      </div>
      <div
        className="workspace-overview"
        role="region"
        aria-label="Network metrics"
      >
        <Statistic
          title="Registered UEs"
          value={registered}
          suffix={`/ ${scenario.ues.length}`}
        />
        <Statistic
          title="Active PDU sessions"
          value={snapshot?.sessions.filter((s) => s.active).length ?? 0}
        />
        <Statistic
          title="Echo packets delivered"
          value={snapshot?.sessions.reduce((n, s) => n + s.delivered, 0) ?? 0}
        />
        <Statistic
          title="Simulation time"
          value={(snapshot?.time_ms ?? 0) / 1000}
          precision={3}
          suffix="s virtual"
        />
        <div className="overview-model">
          <Tag color="blue">
            {radio.model === "Legacy" ? "Legacy RF" : `3GPP ${radio.model}`}
          </Tag>
          <span>Seed {scenario.seed} · deterministic</span>
        </div>
        <div className="mobile-panel-actions">
          <ControlButton onClick={() => setSetupOpen(true)}>
            <Layers size={16} /> Network setup
          </ControlButton>
          <ControlButton onClick={() => setInspectorOpen(true)}>
            <Settings2 size={16} /> Entity inspector
          </ControlButton>
        </div>
      </div>
      <Layout.Content className="workspace">
        <WorkspacePanel
          side="left"
          open={setupOpen}
          onClose={() => setSetupOpen(false)}
        >
          <Card size="small" className="location-section">
            <div className="section-label">
              <span>STUDY AREA</span>
              <MapPin size={14} />
            </div>
            <form onSubmit={search} className="search-box">
              <Search size={16} />
              <Input
                aria-label="Search location"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search city or lon, lat"
              />
              <ControlButton
                aria-label="Find location"
                disabled={searching}
                type="submit"
              >
                {searching ? "…" : "↵"}
              </ControlButton>
            </form>
            {places.length > 0 && (
              <div className="place-results">
                {places.map((p, i) => (
                  <ControlButton
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
                  </ControlButton>
                ))}
              </div>
            )}
            <label>
              RF model{" "}
              <SelectField
                aria-label="RF model"
                value={radio.model}
                onChange={(e) => {
                  const model = e.target.value as typeof radio.model;
                  apply({
                    ...scenario,
                    radio: { ...radio, model },
                    gnodebs: scenario.gnodebs.map((g) => ({
                      ...g,
                      height_m: model === "UMi" ? 10 : 25,
                    })),
                  });
                }}
              >
                {["Legacy", "UMi", "UMa"].map((m) => (
                  <option key={m}>{m}</option>
                ))}
              </SelectField>
            </label>
            <Checkbox
              checked={radio.shadow_fading}
              onChange={(e) =>
                apply({
                  ...scenario,
                  radio: { ...radio, shadow_fading: e.target.checked },
                })
              }
            >
              {" "}
              Seeded shadow fading
            </Checkbox>
            <label>
              Duration (virtual s){" "}
              <Input
                aria-label="Experiment duration"
                type="number"
                min={1}
                max={3600}
                value={radio.duration_ms / 1000}
                onChange={(e) => {
                  const n = Number(e.target.value);
                  if (n >= 1 && n <= 3600)
                    apply({
                      ...scenario,
                      radio: { ...radio, duration_ms: n * 1000 },
                    });
                }}
              />
            </label>
            <div className="area-meta">
              <span>Center</span>
              <span>
                {scenario.center[1].toFixed(4)}, {scenario.center[0].toFixed(4)}
              </span>
            </div>
            <ControlButton
              className="load-buildings"
              disabled={loadingBuildings || pending}
              onClick={loadBuildings}
            >
              <Box size={14} />
              {loadingBuildings ? "Loading geometry…" : "Load OSM buildings"}
              <span>{scenario.buildings.length}</span>
            </ControlButton>
          </Card>
          <Card size="small" className="deployment-section">
            <div className="section-label">
              <span>DEPLOY NETWORK</span>
              <Plus size={14} />
            </div>
            <div className="deploy-buttons">
              <ControlButton
                className={mode === "gnodeb" ? "chosen" : ""}
                onClick={() => setMode(mode === "gnodeb" ? "select" : "gnodeb")}
                disabled={pending}
              >
                <Antenna size={21} />
                <strong>gNodeB</strong>
                <span>Base station</span>
              </ControlButton>
              <ControlButton
                className={mode === "ue" ? "chosen" : ""}
                onClick={() => setMode(mode === "ue" ? "select" : "ue")}
                disabled={pending}
              >
                <Smartphone size={21} />
                <strong>UE</strong>
                <span>User equipment</span>
              </ControlButton>
            </div>
            <p className="hint">Choose an entity, then place it on the map.</p>
          </Card>
          <Card size="small" className="entities-section">
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
          </Card>
          <Card size="small" className="layer-section">
            <div className="section-label">
              <span>MAP LAYERS</span>
              <Layers size={14} />
            </div>
            {[
              ["Coverage estimate", coverage, setCoverage],
              ["Building footprints", buildings, setBuildings],
              ["Serving-cell links", links, setLinks],
            ].map(([label, value, setter]) => (
              <div className="toggle-row" key={label as string}>
                <span>{label as string}</span>
                <Switch
                  size="small"
                  aria-label={label as string}
                  checked={value as boolean}
                  onChange={(checked) =>
                    (setter as React.Dispatch<React.SetStateAction<boolean>>)(
                      checked,
                    )
                  }
                />
              </div>
            ))}
          </Card>
          <div className="sidebar-footer">
            <FlaskConical size={16} />
            <span>
              Built for exploration.
              <br />
              <strong>Every model has boundaries.</strong>
            </span>
            <ControlButton
              title="Read model limitations"
              onClick={() => setModelOpen(true)}
            >
              <Info size={16} />
            </ControlButton>
          </div>
        </WorkspacePanel>
        <div className="center-panel">
          <div className="map-header">
            <div>
              <span className="tab-active">
                <MapPin size={14} /> Network map
              </span>
              <span className="map-subtitle">1.7 × 1.7 km study grid</span>
            </div>
            <div>
              <ControlButton
                className={mode === "select" ? "active" : ""}
                title="Select and drag entities"
                onClick={() => setMode("select")}
              >
                <MousePointer2 size={15} />
              </ControlButton>
              <ControlButton
                className={mode === "move" ? "active" : ""}
                title="Move selected entity"
                disabled={!entity}
                onClick={() => setMode("move")}
              >
                <Move size={15} />
              </ControlButton>
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
              setInspectorOpen(true);
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
              <ControlButton
                title="Reset simulation"
                aria-label="Reset simulation"
                disabled={pending}
                onClick={() => apply({ ...scenario })}
              >
                <RotateCcw size={15} />
              </ControlButton>
              <ControlButton
                title="Advance 10 milliseconds"
                aria-label="Step simulation"
                disabled={pending || !!error || !!completed}
                onClick={() => command("step")}
              >
                <SkipForward size={15} />
              </ControlButton>
              <span className="sim-clock">
                {((snapshot?.time_ms ?? 0) / 1000).toFixed(3)}
                <small> s</small>
              </span>
            </div>
            <label className="seed">
              SEED{" "}
              <Input
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
            <label className="speed">
              Playback{" "}
              <SelectField
                aria-label="Simulation speed"
                value={speed}
                onChange={(e) => {
                  const speed = Number(e.target.value);
                  setSpeed(speed);
                  worker.current?.postMessage({
                    type: "speed",
                    speed,
                    revision: activeRevision.current,
                  } satisfies WorkerCommand);
                }}
              >
                {[0.1, 1, 5, 10].map((s) => (
                  <option key={s} value={s}>
                    {s}×
                  </option>
                ))}
              </SelectField>
            </label>
          </div>
          <section className="timeline">
            <div className="timeline-heading">
              <h2>
                <Activity size={16} /> Protocol timeline{" "}
                <span>
                  {snapshot?.events.length ?? 0}
                  {events.length > 500 ? " · latest 500 shown" : ""}
                </span>
              </h2>
              <div className="timeline-filters">
                <SelectField
                  aria-label="Filter timeline by UE"
                  value={ueFilter}
                  onChange={(e) => setUeFilter(e.target.value)}
                >
                  <option value="all">All UEs</option>
                  {scenario.ues.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.name}
                    </option>
                  ))}
                </SelectField>
                {[
                  "all",
                  "NAS",
                  "NGAP",
                  "5GSM",
                  "GTP-U",
                  "IPv4",
                  "RADIO",
                  "STATE",
                ].map((f) => (
                  <ControlButton
                    className={eventFilter === f ? "active" : ""}
                    key={f}
                    onClick={() => setEventFilter(f)}
                  >
                    {f === "all" ? "All events" : f}
                  </ControlButton>
                ))}
                <ControlButton
                  title="Export event log"
                  aria-label="Export event log"
                  onClick={() =>
                    download("ranterra-events.json", snapshot?.events ?? [])
                  }
                >
                  <ArrowDownToLine size={14} />
                </ControlButton>
              </div>
            </div>
            <div className="timeline-body">
              <div className="event-list">
                {!events.length ? (
                  <Empty
                    className="empty-events"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={
                      <>
                        <strong>Your network is ready to talk.</strong>
                        <p>
                          Run the simulation to inspect real NAS and NGAP
                          messages.
                        </p>
                      </>
                    }
                  />
                ) : (
                  events.slice(-500).map((e) => (
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
        <WorkspacePanel
          side="right"
          open={inspectorOpen}
          onClose={() => setInspectorOpen(false)}
        >
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
                  <Badge
                    className="entity-state"
                    status={
                      gnodeb || ueStatus?.state === "Registered"
                        ? "success"
                        : [
                              "RegistrationRejected",
                              "NoCoverage",
                              "RegistrationUnconfirmed",
                            ].includes(ueStatus?.state ?? "")
                          ? "error"
                          : ueStatus?.state && ueStatus.state !== "Idle"
                            ? "processing"
                            : "default"
                    }
                    text={
                      gnodeb
                        ? (snapshot?.gnodebs[entity.id] ?? "Ready")
                        : prettyState(ueStatus?.state)
                    }
                  />
                </div>
              </div>
              <Card size="small" className="inspector-section">
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
                    <Checkbox
                      checked={gnodeb.antenna?.sector ?? false}
                      onChange={(e) =>
                        apply({
                          ...scenario,
                          gnodebs: scenario.gnodebs.map((g) =>
                            g.id === selected
                              ? {
                                  ...g,
                                  antenna: {
                                    sector: e.target.checked,
                                    azimuth_deg: g.antenna?.azimuth_deg ?? 0,
                                    downtilt_deg: g.antenna?.downtilt_deg ?? 0,
                                    resource_blocks:
                                      g.antenna?.resource_blocks ?? 106,
                                    numerology: g.antenna?.numerology ?? 1,
                                  },
                                }
                              : g,
                          ),
                        })
                      }
                    >
                      {" "}
                      Sector antenna
                    </Checkbox>
                    {(
                      [
                        ["azimuth_deg", "Azimuth", 0, 360, 0],
                        ["downtilt_deg", "Downtilt", -20, 90, 0],
                        ["resource_blocks", "Resource blocks", 1, 275, 106],
                        ["numerology", "Numerology", 0, 4, 1],
                      ] as const
                    ).map(([key, label, min, max, fallback]) => (
                      <RadioField
                        key={`${gnodeb.id}-${key}`}
                        label={label}
                        unit=""
                        min={min}
                        max={max}
                        value={gnodeb.antenna?.[key] ?? fallback}
                        onChange={(value) =>
                          apply({
                            ...scenario,
                            gnodebs: scenario.gnodebs.map((g) =>
                              g.id === selected
                                ? {
                                    ...g,
                                    antenna: {
                                      sector: false,
                                      azimuth_deg: 0,
                                      downtilt_deg: 0,
                                      resource_blocks: 106,
                                      numerology: 1,
                                      ...g.antenna,
                                      [key]: value,
                                    },
                                  }
                                : g,
                            ),
                          })
                        }
                      />
                    ))}
                    <div className="radio-footnote">
                      <Info size={13} />
                      <span>
                        Per-resource-element power proxy. Configure antenna and
                        resource allocation below.
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
                      <span>SINR</span>
                      <strong>{ueStatus?.sinr_db?.toFixed(1) ?? "—"} dB</strong>
                    </div>
                    <div className="detail-row">
                      <span>Registration</span>
                      <strong>
                        {ueStatus?.state === "Registered"
                          ? "REGISTERED · NIA2 / NEA2"
                          : prettyState(ueStatus?.state)}
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>AMF context</span>
                      <strong>
                        {snapshot?.amf[entity.id] ?? "Not created"}
                      </strong>
                    </div>
                    {ueStatus?.failure && (
                      <p role="status">{ueStatus.failure}</p>
                    )}
                    <div className="detail-row">
                      <span>Registration latency</span>
                      <strong>
                        {ueStatus?.registration_latency_ms ?? "—"} ms virtual
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>PDU session</span>
                      <strong>
                        {session?.active ? session.ip.join(".") : "Inactive"}
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>Echo delivered / dropped</span>
                      <strong>
                        {session?.delivered ?? 0} / {session?.dropped ?? 0}
                      </strong>
                    </div>
                    <div className="detail-row">
                      <span>Last echo RTT</span>
                      <strong>{session?.last_rtt_ms ?? "—"} ms virtual</strong>
                    </div>
                    <ControlButton
                      disabled={
                        ueStatus?.state !== "Registered" ||
                        !!session?.active ||
                        sessionPending ||
                        !!(
                          sessionFailure &&
                          (sessionFailure.retry_at_ms === null ||
                            (snapshot?.time_ms ?? 0) <
                              sessionFailure.retry_at_ms)
                        )
                      }
                      onClick={() => {
                        sessionAction("establish");
                        command("play");
                      }}
                    >
                      Establish PDU session
                    </ControlButton>
                    {sessionPending && (
                      <p role="status">
                        Session signaling pending · virtual time
                      </p>
                    )}
                    {sessionFailure && (
                      <p role="status">
                        {sessionFailure.domain} cause {sessionFailure.cause}:{" "}
                        {sessionFailure.reason}
                        {sessionFailure.retry_at_ms !== null
                          ? `; retry at ${sessionFailure.retry_at_ms} virtual ms`
                          : ""}
                      </p>
                    )}
                    <label>
                      Echo loss (%)
                      <Input
                        aria-label="Echo loss percent"
                        type="number"
                        min={0}
                        max={100}
                        value={lossPercent}
                        onChange={(e) =>
                          setLossPercent(
                            Math.max(0, Math.min(100, Number(e.target.value))),
                          )
                        }
                      />
                    </label>
                    <ControlButton
                      disabled={!session?.active || sessionPending}
                      onClick={() => {
                        sessionAction("echo");
                        command("play");
                      }}
                    >
                      Send echo
                    </ControlButton>
                    <ControlButton
                      disabled={!session?.active || sessionPending}
                      onClick={() => {
                        sessionAction("release");
                        command("play");
                      }}
                    >
                      Release session
                    </ControlButton>
                    <ControlButton
                      disabled={
                        ueStatus?.state !== "Registered" ||
                        sessionPending ||
                        !!session?.active
                      }
                      onClick={() => sessionAction("deregister")}
                    >
                      Deregister UE
                    </ControlButton>
                    <ControlButton
                      onClick={() =>
                        setMode(mode === "trajectory" ? "select" : "trajectory")
                      }
                    >
                      {mode === "trajectory"
                        ? "Finish trajectory"
                        : "Draw trajectory"}
                    </ControlButton>
                    <fieldset className="session-fields">
                      <legend>Signaling laboratory (edits reset run)</legend>
                      <label>
                        Session hop delay (virtual ms)
                        <Input
                          aria-label="Signaling delay"
                          type="number"
                          min={1}
                          max={1000}
                          value={signaling?.delay_ms ?? 20}
                          onChange={(e) => {
                            const value = Number(e.target.value);
                            if (value >= 1 && value <= 1000)
                              changeSignaling({ delay_ms: value });
                          }}
                        />
                      </label>
                      <label>
                        Drop message
                        <SelectField
                          aria-label="Signaling loss"
                          value={signaling?.drop ?? "None"}
                          onChange={(e) =>
                            changeSignaling({ drop: e.target.value })
                          }
                        >
                          {[
                            "None",
                            "RegistrationRequest",
                            "AuthenticationRequest",
                            "AuthenticationResponse",
                            "SecurityModeCommand",
                            "SecurityModeComplete",
                            "RegistrationAccept",
                            "RegistrationComplete",
                            "N2SetupResponse",
                            "ReleaseCommand",
                            "ReleaseComplete",
                            "N2ReleaseResponse",
                          ].map((v) => (
                            <option key={v}>{v}</option>
                          ))}
                        </SelectField>
                      </label>
                      <label>
                        Initial messages dropped
                        <Input
                          aria-label="Signaling drop count"
                          type="number"
                          min={0}
                          max={5}
                          value={signaling?.count ?? 0}
                          onChange={(e) => {
                            const value = Number(e.target.value);
                            if (
                              Number.isInteger(value) &&
                              value >= 0 &&
                              value <= 5
                            )
                              changeSignaling({ count: value });
                          }}
                        />
                      </label>
                      <Checkbox
                        checked={signaling?.reject_n2 ?? false}
                        onChange={(e) =>
                          changeSignaling({ reject_n2: e.target.checked })
                        }
                      >
                        Reject N2 radio resources
                      </Checkbox>
                    </fieldset>
                    <fieldset className="session-fields">
                      <legend>Requested data service (edits reset run)</legend>
                      <label>
                        DNN
                        <Input
                          aria-label="UE DNN"
                          key={`dnn-${ue?.id}-${ueRequest.dnn}`}
                          defaultValue={ueRequest.dnn}
                          maxLength={99}
                          onBlur={(e) => {
                            if (ue && e.target.value !== ueRequest.dnn)
                              apply({
                                ...scenario,
                                ues: scenario.ues.map((u) =>
                                  u.id === ue.id
                                    ? {
                                        ...u,
                                        session: {
                                          ...ueRequest,
                                          dnn: e.target.value,
                                        },
                                      }
                                    : u,
                                ),
                              });
                          }}
                        />
                      </label>
                      <label>
                        SST
                        <Input
                          aria-label="UE SST"
                          type="number"
                          min={1}
                          max={255}
                          key={`sst-${ue?.id}-${ueRequest.snssai.sst}`}
                          defaultValue={ueRequest.snssai.sst}
                          onBlur={(e) => {
                            if (
                              ue &&
                              Number(e.target.value) !== ueRequest.snssai.sst
                            )
                              apply({
                                ...scenario,
                                ues: scenario.ues.map((u) =>
                                  u.id === ue.id
                                    ? {
                                        ...u,
                                        session: {
                                          ...ueRequest,
                                          snssai: {
                                            ...ueRequest.snssai,
                                            sst: Number(e.target.value),
                                          },
                                        },
                                      }
                                    : u,
                                ),
                              });
                          }}
                        />
                      </label>
                      <label>
                        SD (6 hex digits, optional)
                        <Input
                          aria-label="UE SD"
                          key={`sd-${ue?.id}-${ueRequest.snssai.sd}`}
                          defaultValue={
                            ueRequest.snssai.sd
                              ?.map((n) => n.toString(16).padStart(2, "0"))
                              .join("") ?? ""
                          }
                          maxLength={6}
                          onBlur={(e) => {
                            const value = e.target.value;
                            if (!/^(?:[0-9a-fA-F]{6})?$/.test(value)) {
                              setNotice(
                                "SD requires exactly six hex digits or an empty value.",
                              );
                              return;
                            }
                            const sd = value
                              ? value.match(/../g)!.map((h) => parseInt(h, 16))
                              : null;
                            if (
                              ue &&
                              JSON.stringify(sd) !==
                                JSON.stringify(ueRequest.snssai.sd)
                            )
                              apply({
                                ...scenario,
                                ues: scenario.ues.map((u) =>
                                  u.id === ue.id
                                    ? {
                                        ...u,
                                        session: {
                                          ...ueRequest,
                                          snssai: { ...ueRequest.snssai, sd },
                                        },
                                      }
                                    : u,
                                ),
                              });
                          }}
                        />
                      </label>
                      {session?.active && (
                        <p>
                          Active: {session.dnn} · SST {session.sst} · SD{" "}
                          {session.sd
                            ?.map((n) => n.toString(16).padStart(2, "0"))
                            .join("") ?? "none"}
                        </p>
                      )}
                    </fieldset>
                    <label>
                      Speed (m/s)
                      <Input
                        aria-label="UE velocity"
                        type="number"
                        min={0}
                        max={100}
                        defaultValue={ue?.mobility?.speed_mps ?? 0}
                        key={`speed-${ue?.id}`}
                        onBlur={(e) => {
                          const speed = Number(e.target.value);
                          if (
                            speed >= 0 &&
                            speed <= 100 &&
                            speed !== (ue?.mobility?.speed_mps ?? 0)
                          )
                            apply({
                              ...scenario,
                              ues: scenario.ues.map((u) =>
                                u.id === selected
                                  ? {
                                      ...u,
                                      mobility: {
                                        waypoints: u.mobility?.waypoints ?? [],
                                        speed_mps: speed,
                                      },
                                    }
                                  : u,
                              ),
                            });
                        }}
                      />
                    </label>
                    <label>
                      Laboratory fault
                      <SelectField
                        aria-label="Laboratory fault"
                        value={ue?.fault ?? "None"}
                        onChange={(e) =>
                          apply({
                            ...scenario,
                            ues: scenario.ues.map((u) =>
                              u.id === selected
                                ? { ...u, fault: e.target.value }
                                : u,
                            ),
                          })
                        }
                      >
                        {[
                          "None",
                          "UnknownSubscriber",
                          "InvalidCredentials",
                          "InvalidAutn",
                          "SqnOutOfSync",
                          "IncorrectResponse",
                          "InvalidMac",
                          "InvalidCounter",
                          "Timeout",
                        ].map((f) => (
                          <option key={f}>{f}</option>
                        ))}
                      </SelectField>
                    </label>
                    <p className="hint">
                      SqnOutOfSync starts the synthetic USIM at SQN 100 and
                      exercises authenticated resynchronization. Cell selection
                      uses strongest estimated RSRP above −120 dBm by default.
                      Mobility uses 100 ms ticks and A3-style reassociation.
                      Protocol handover is unsupported.
                    </p>
                  </>
                )}
              </Card>
              <Card size="small" className="inspector-section">
                <div className="section-label">
                  POSITION <MapPin size={13} />
                </div>
                <label className="entity-name-field">
                  Entity name
                  <Input
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
                    <Input
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
                    <Input
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
                <ControlButton
                  className="full-button"
                  onClick={() => setMode(mode === "move" ? "select" : "move")}
                >
                  <Move size={14} />
                  {mode === "move"
                    ? "Click destination on map"
                    : "Reposition on map"}
                </ControlButton>
              </Card>
              <Card size="small" className="inspector-section">
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
                      <strong>
                        {snapshot?.ues.filter(
                          (u) =>
                            u.serving_cell === gnodeb.id &&
                            u.state === "Registered",
                        ).length ?? 0}
                      </strong>
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
                      Synthetic laboratory subscribers only. MILENAGE 5G-AKA,
                      NIA2 integrity and NEA2 ciphering. Never import real
                      credentials.
                    </span>
                  </div>
                )}
              </Card>
              <ControlButton
                className="delete-entity"
                onClick={remove}
                disabled={pending}
              >
                <Trash2 size={14} /> Delete {gnodeb ? "gNodeB" : "UE"}
              </ControlButton>
            </>
          ) : (
            <Empty
              className="inspector-empty"
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <>
                  <h2>Inspect your network</h2>
                  <p>Select a base station or UE on the map.</p>
                </>
              }
            />
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
        </WorkspacePanel>
      </Layout.Content>
      <footer className="statusbar">
        <span>
          <span className="live-dot" /> Browser-native simulation{" "}
          <span className="muted">/</span> No application backend
        </span>
        <span>
          OxiRush NAS + NGAP <span className="muted">/</span> Approximate RF{" "}
          <span className="muted">/</span>
          <ControlButton onClick={() => setModelOpen(true)}>
            v1.0 candidate <Info size={11} />
          </ControlButton>
        </span>
      </footer>
      {error && (
        <div className="error-banner" role="alert">
          <ShieldAlert size={20} />
          <span>{error}</span>
          {snapshot && error.startsWith("Rust/WASM engine:") && (
            <ControlButton onClick={() => setError("")}>
              Continue current run
            </ControlButton>
          )}
          <ControlButton
            onClick={() => {
              setError("");
              setPending(true);
              setWorkerEpoch((n) => n + 1);
            }}
          >
            Recover worker
          </ControlButton>
          <ControlButton onClick={() => apply(structuredClone(demo))}>
            Reset to demo
          </ControlButton>
        </div>
      )}
      {snapshot?.event_limit_reached && (
        <Alert
          role="status"
          className="toast"
          type="warning"
          showIcon
          title="Trace limit reached (16,384 events). Reset before recording more events; computation continues."
        />
      )}
      <ResearchModal
        open={policiesOpen}
        title="Session policies"
        className="research-modal session-policy-modal"
        width={800}
        footer={null}
        getContainer={false}
        onCancel={() => setPoliciesOpen(false)}
        closable={{ "aria-label": "Close policies" }}
      >
        <h2>Network session policies</h2>
        {error && <p role="alert">{error}</p>}
        <p>
          Each row permits one DNN / S-NSSAI pair. Capacity counts active
          sessions across UEs. Zero denies new sessions. Applying changes resets
          execution.
        </p>
        <form
          key={JSON.stringify(policies)}
          onSubmit={(e) => {
            e.preventDefault();
            const data = new FormData(e.currentTarget);
            const next: SessionPolicy[] = [];
            for (let i = 0; i < 8; i++) {
              const dnn = String(data.get(`dnn-${i}`) ?? "").trim();
              if (!dnn) continue;
              const hex = String(data.get(`sd-${i}`) ?? "").trim();
              if (!/^(?:[0-9a-fA-F]{6})?$/.test(hex)) {
                setNotice("SD requires six hex digits or an empty value.");
                return;
              }
              next.push({
                request: {
                  dnn,
                  snssai: {
                    sst: Number(data.get(`sst-${i}`)),
                    sd: hex
                      ? hex.match(/../g)!.map((h) => parseInt(h, 16))
                      : null,
                  },
                },
                max_sessions: Number(data.get(`capacity-${i}`)),
              });
            }
            apply({ ...scenario, session_policies: next });
          }}
        >
          <table>
            <thead>
              <tr>
                <th>DNN (blank removes)</th>
                <th>SST</th>
                <th>SD (hex)</th>
                <th>Capacity</th>
              </tr>
            </thead>
            <tbody>
              {Array.from({ length: 8 }, (_, i) => (
                <tr key={i}>
                  <td>
                    <Input
                      aria-label={`Policy ${i + 1} DNN`}
                      name={`dnn-${i}`}
                      defaultValue={policies[i]?.request.dnn ?? ""}
                      maxLength={99}
                    />
                  </td>
                  <td>
                    <Input
                      aria-label={`Policy ${i + 1} SST`}
                      name={`sst-${i}`}
                      type="number"
                      min={1}
                      max={255}
                      defaultValue={policies[i]?.request.snssai.sst ?? 1}
                    />
                  </td>
                  <td>
                    <Input
                      aria-label={`Policy ${i + 1} SD`}
                      name={`sd-${i}`}
                      maxLength={6}
                      defaultValue={
                        policies[i]?.request.snssai.sd
                          ?.map((n) => n.toString(16).padStart(2, "0"))
                          .join("") ?? ""
                      }
                    />
                  </td>
                  <td>
                    <Input
                      aria-label={`Policy ${i + 1} capacity`}
                      name={`capacity-${i}`}
                      type="number"
                      min={0}
                      max={128}
                      defaultValue={policies[i]?.max_sessions ?? 128}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <ControlButton type="submit" disabled={pending}>
            Apply policies
          </ControlButton>
        </form>
      </ResearchModal>
      <ResearchModal
        open={resultsOpen}
        title="Experiment results"
        className="research-modal results-modal"
        width={920}
        footer={null}
        getContainer={false}
        onCancel={() => setResultsOpen(false)}
        closable={{ "aria-label": "Close results" }}
      >
        <p>
          {scenario.name} · Seed {scenario.seed} · RF {radio.model} ·{" "}
          {(snapshot?.time_ms ?? 0) / 1000} s virtual time
        </p>
        <div className="results-statistics">
          <Statistic
            title="Registered / rejected / deployed"
            value={`${registered} / ${failed} / ${scenario.ues.length}`}
          />
          <Statistic
            title="Registration success · all deployed UEs"
            value={
              scenario.ues.length
                ? (100 * latencies.length) / scenario.ues.length
                : 0
            }
            precision={1}
            suffix="%"
          />
          <Statistic
            title="Mean successful registration latency"
            value={latency?.toFixed(1) ?? "—"}
            suffix="ms virtual"
          />
          <Statistic
            title="Active PDU sessions"
            value={snapshot?.sessions.filter((s) => s.active).length ?? 0}
          />
          <Statistic
            title="Echo delivered / dropped"
            value={`${snapshot?.sessions.reduce((n, s) => n + s.delivered, 0) ?? 0} / ${snapshot?.sessions.reduce((n, s) => n + s.dropped, 0) ?? 0}`}
          />
        </div>
        <Table
          size="small"
          pagination={false}
          rowKey="id"
          dataSource={scenario.gnodebs}
          columns={[
            { title: "Cell", dataIndex: "name" },
            {
              title: "Associated UEs",
              render: (_, g) =>
                snapshot?.ues.filter((u) => u.serving_cell === g.id).length ??
                0,
            },
          ]}
        />
        <p>
          Protocol handovers: unsupported. SINR uses full-load co-channel
          interference; RSRP is a power-allocation proxy. New runs use
          configured virtual session delays; legacy recordings preserve atomic
          session control. Deregistration remains atomic.
        </p>
        {baseline && (
          <p>
            Prior comparison: {baseline.registered} registered;{" "}
            {baseline.latency?.toFixed(1) ?? "—"} ms mean registration latency.
          </p>
        )}
        <ControlButton onClick={() => setBaseline({ registered, latency })}>
          Keep comparison
        </ControlButton>
        <p>
          Experiment checkpoints include ordered commands and reconstruct
          protocol state without storing keys. Export and save pause playback.
        </p>
        <ControlButton
          disabled={pending}
          onClick={() =>
            worker.current?.postMessage({
              type: "recording",
              purpose: "export",
              revision: activeRevision.current,
            } satisfies WorkerCommand)
          }
        >
          Export experiment
        </ControlButton>
        <ControlButton
          disabled={pending}
          onClick={() =>
            worker.current?.postMessage({
              type: "recording",
              purpose: "save",
              revision: activeRevision.current,
            } satisfies WorkerCommand)
          }
        >
          Save run
        </ControlButton>
        <ControlButton disabled={pending} onClick={() => void restoreRun()}>
          Restore saved run
        </ControlButton>
        <ControlButton
          onClick={() => {
            importPurpose.current = "compare";
            file.current?.click();
          }}
        >
          Compare experiment
        </ControlButton>
        {comparison && (
          <table aria-label="Experiment comparison">
            <caption>{comparison.name} vs current run</caption>
            <thead>
              <tr>
                <th>Metric</th>
                <th>Imported run</th>
                <th>Current run</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Virtual time (ms)</td>
                <td>{comparison.snapshot.time_ms}</td>
                <td>{snapshot?.time_ms}</td>
              </tr>
              <tr>
                <td>Registered UEs</td>
                <td>
                  {
                    comparison.snapshot.ues.filter(
                      (u) => u.state === "Registered",
                    ).length
                  }
                </td>
                <td>{registered}</td>
              </tr>
              <tr>
                <td>Echo delivered</td>
                <td>
                  {comparison.snapshot.sessions.reduce(
                    (n, s) => n + s.delivered,
                    0,
                  )}
                </td>
                <td>
                  {snapshot?.sessions.reduce((n, s) => n + s.delivered, 0)}
                </td>
              </tr>
              <tr>
                <td>Echo dropped</td>
                <td>
                  {comparison.snapshot.sessions.reduce(
                    (n, s) => n + s.dropped,
                    0,
                  )}
                </td>
                <td>{snapshot?.sessions.reduce((n, s) => n + s.dropped, 0)}</td>
              </tr>
            </tbody>
          </table>
        )}
        <ControlButton onClick={metricsCSV}>Export metrics CSV</ControlButton>
        <ControlButton
          onClick={() =>
            download("ranterra-results.json", { scenario, snapshot })
          }
        >
          Export results JSON
        </ControlButton>
      </ResearchModal>
      {notice && (
        <Alert
          className="toast"
          role="status"
          showIcon
          type="info"
          title={notice}
          closable={{ "aria-label": "Dismiss notification" }}
          afterClose={() => setNotice("")}
        />
      )}
      <ResearchModal
        open={modelOpen}
        title="Models and assumptions"
        className="research-modal"
        width={720}
        footer={null}
        getContainer={false}
        onCancel={() => setModelOpen(false)}
        closable={{ "aria-label": "Close model information" }}
      >
        <span className="eyebrow">KNOW YOUR MODEL</span>
        <h2>A playground, with clear boundaries.</h2>
        <p>
          RANterra executes a deterministic Rust engine in a Web Worker. NAS and
          NGAP messages are encoded and decoded by OxiRush, compiled to
          WebAssembly.
        </p>
        <h3>Radio propagation</h3>
        <code>
          Selected model: {radio.model} · TR 38.901 v18.0.0 subset or legacy
          log-distance
          <br />
          RSRP ≈ Ptx + gain − 10 log₁₀(12 × N_RB) − PL − antenna/shadow loss
        </code>
        <p>
          UMi and UMa use LOS/NLOS path loss with 1.5 m UE height, 10–5000 m
          horizontal distance and 0.5–100 GHz carrier. UMi requires 10 m
          antennas, UMa 25 m. Outside that range the research model reports no
          usable coverage. Shadow fading is seeded but not spatially correlated.
          No terrain, diffraction, fast fading or PHY scheduling.
        </p>
        <h3>Buildings are a coarse proxy</h3>
        <p>
          In UMi/UMa, footprint intersections select NLOS; missing geometry
          assumes LOS. Legacy mode adds 12 dB per footprint, capped at 36 dB.
          Only closed simple ways are loaded, up to 600 buildings. Heights,
          materials, holes and multipolygon relations are not modeled. Missing
          buildings mean no obstruction loss.
        </p>
        <h3>Authenticated laboratory registration</h3>
        <p>
          Registration Request → 5G-AKA Authentication → Security Mode →
          Registration Accept / Complete. Separate UE and AMF NAS security
          contexts enforce NIA2 and NEA2. Public deterministic synthetic
          credentials and null-scheme SUCI are for laboratory use only. RRC, NG
          setup and native SCTP remain unsupported.
        </p>
        <h3>Local and reproducible</h3>
        <p>
          The seed controls virtual link delays (10–30 ms per hop). Editing the
          scenario resets the experiment. Search, tiles and new OSM geometry
          need internet; loaded geometry, protocol execution, coverage and JSON
          exports work locally. 6G radio models and AI controllers are extension
          points, not implemented features.
        </p>
        <ControlButton
          className="run-button"
          onClick={() => setModelOpen(false)}
        >
          Back to the experiment <ChevronRight size={15} />
        </ControlButton>
      </ResearchModal>
      <input
        ref={file}
        type="file"
        accept=".json,application/json"
        hidden
        onChange={importFile}
      />
    </Layout>
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
        <Input
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
        <ControlButton
          title="Copy message bytes"
          onClick={() =>
            void navigator.clipboard
              .writeText(event.hex)
              .then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 1200);
              })
              .catch(() => setCopied(false))
          }
        >
          {copied ? <Check size={13} /> : <Copy size={13} />}
        </ControlButton>
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
