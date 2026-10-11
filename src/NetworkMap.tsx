import { useEffect, useRef, useState } from "react";
import * as maplibregl from "maplibre-gl";
import { MapLibreOverlay } from "@deck.gl/maplibre";
import { LineLayer, PolygonLayer, ScatterplotLayer } from "@deck.gl/layers";
import {
  Crosshair,
  Layers,
  Minus,
  Plus,
  LocateFixed,
  MapPin,
} from "lucide-react";
import type { Point, Scenario, Snapshot, ToolMode } from "./types";
import "maplibre-gl/dist/maplibre-gl.css";
import mapWorkerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
maplibregl.setWorkerUrl(mapWorkerUrl);
interface Props {
  scenario: Scenario;
  snapshot: Snapshot | null;
  selected: number | null;
  mode: ToolMode;
  coverage: boolean;
  buildings: boolean;
  links: boolean;
  onSelect: (id: number) => void;
  onPlace: (p: Point) => void;
  onMove: (id: number, p: Point) => void;
  onCenter: (p: Point) => void;
}
export function signalColor(value: number): [number, number, number, number] {
  if (value >= -80) return [38, 145, 108, 115];
  if (value >= -95) return [108, 177, 104, 100];
  if (value >= -110) return [209, 192, 106, 82];
  return [210, 138, 113, 55];
}
export default function NetworkMap(props: Props) {
  const container = useRef<HTMLDivElement>(null),
    map = useRef<maplibregl.Map | null>(null),
    overlay = useRef<MapLibreOverlay | null>(null),
    latest = useRef(props);
  latest.current = props;
  const [mapError, setMapError] = useState(""),
    [position, setPosition] = useState<Point>(props.scenario.center);
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    if (!container.current) return;
    const m = new maplibregl.Map({
      container: container.current,
      center: props.scenario.center,
      zoom: 15.5,
      maxZoom: 19,
      minZoom: 3,
      attributionControl: { compact: false },
      style: {
        version: 8,
        sources: {
          basemap: {
            type: "raster",
            tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
            tileSize: 256,
            attribution:
              '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
          },
        },
        layers: [
          {
            id: "background",
            type: "background",
            paint: { "background-color": "#e6e9e3" },
          },
          {
            id: "base",
            type: "raster",
            source: "basemap",
            paint: { "raster-saturation": -0.65, "raster-opacity": 0.72 },
          },
        ],
      },
    });
    map.current = m;
    const deck = new MapLibreOverlay({ interleaved: false, layers: [] });
    overlay.current = deck;
    m.addControl(deck);
    m.on("load", () => setLoaded(true));
    m.on("error", (e) =>
      setMapError(
        `Map asset error: ${e.error.message.slice(0, 100)}. Local simulation remains available.`,
      ),
    );
    m.on("click", (e) => latest.current.onPlace([e.lngLat.lng, e.lngLat.lat]));
    m.on("moveend", () => {
      const p = m.getCenter();
      setPosition([p.lng, p.lat]);
    });
    const resize = new ResizeObserver(() => m.resize());
    resize.observe(container.current);
    return () => {
      resize.disconnect();
      m.remove();
      map.current = null;
    };
  }, []);
  useEffect(() => {
    map.current?.flyTo({
      center: props.scenario.center,
      zoom: 15.5,
      duration: 800,
    });
  }, [props.scenario.center]);
  useEffect(() => {
    if (!map.current) return;
    map.current.getCanvas().style.cursor =
      props.mode === "select" ? "grab" : "crosshair";
  }, [props.mode]);
  useEffect(() => {
    if (!loaded || !map.current) return;
    const markers = [
      ...props.scenario.gnodebs.map((g) => ({ ...g, type: "gnodeb" })),
      ...props.scenario.ues.map((u) => ({ ...u, type: "ue" })),
    ].map((entity) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `entity-marker ${entity.type} ${props.selected === entity.id ? "selected" : ""}`;
      button.setAttribute("aria-label", `Select ${entity.name}`);
      const status = props.snapshot?.ues.find((u) => u.id === entity.id);
      button.innerHTML =
        entity.type === "gnodeb"
          ? '<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M12 9v12m-4 0h8M8 11a6 6 0 0 1 0-8m8 0a6 6 0 0 1 0 8M5 14a10 10 0 0 1 0-14m14 0a10 10 0 0 1 0 14"/><circle cx="12" cy="7" r="2"/></svg>'
          : '<span class="ue-dot"></span>';
      const label = document.createElement("span");
      label.className = "marker-label";
      label.textContent =
        entity.type === "gnodeb"
          ? `gNB-${entity.id.toString().padStart(2, "0")}`
          : entity.name;
      button.append(label);
      if (entity.type === "ue" && status?.rsrp != null) {
        const signal = document.createElement("span");
        signal.className = "marker-signal";
        signal.textContent = `${status.rsrp.toFixed(0)} dBm`;
        button.append(signal);
      }
      button.addEventListener("click", (e) => {
        e.stopPropagation();
        latest.current.onSelect(entity.id);
      });
      const marker = new maplibregl.Marker({
        element: button,
        draggable: props.mode === "select",
        anchor: "center",
      })
        .setLngLat([entity.lon, entity.lat])
        .addTo(map.current!);
      marker.on("dragend", () => {
        const p = marker.getLngLat();
        latest.current.onMove(entity.id, [p.lng, p.lat]);
      });
      return marker;
    });
    return () => markers.forEach((m) => m.remove());
  }, [
    props.scenario.gnodebs,
    props.scenario.ues,
    props.selected,
    props.mode,
    props.snapshot?.ues,
    loaded,
  ]);
  useEffect(() => {
    const { scenario, snapshot } = props;
    const cells = (snapshot?.coverage ?? []).map((s) => {
      const dx = 25 / (111320 * Math.cos((s.position[1] * Math.PI) / 180)),
        dy = 25 / 111320;
      return {
        ...s,
        polygon: [
          [s.position[0] - dx, s.position[1] - dy],
          [s.position[0] + dx, s.position[1] - dy],
          [s.position[0] + dx, s.position[1] + dy],
          [s.position[0] - dx, s.position[1] + dy],
        ],
      };
    });
    const associations = (snapshot?.ues ?? []).flatMap((u) => {
      const ue = scenario.ues.find((x) => x.id === u.id),
        g = scenario.gnodebs.find((x) => x.id === u.serving_cell);
      return ue && g
        ? [{ from: [ue.lon, ue.lat] as Point, to: [g.lon, g.lat] as Point }]
        : [];
    });
    overlay.current?.setProps({
      layers: [
        new PolygonLayer({
          id: "coverage",
          data: props.coverage ? cells : [],
          getPolygon: (d) => d.polygon,
          getFillColor: (d) => signalColor(d.rsrp),
          stroked: false,
          pickable: false,
        }),
        new PolygonLayer({
          id: "buildings",
          data: props.buildings ? scenario.buildings : [],
          getPolygon: (d) => d.ring,
          getFillColor: [80, 105, 93, 55],
          getLineColor: [74, 98, 85, 105],
          stroked: true,
          lineWidthMinPixels: 0.5,
        }),
        new ScatterplotLayer({
          id: "cell-radius",
          data: scenario.gnodebs,
          getPosition: (d) => [d.lon, d.lat],
          getRadius: 230,
          filled: false,
          stroked: true,
          getLineColor: [34, 108, 78, 90],
          lineWidthMinPixels: 1,
        }),
        new LineLayer({
          id: "associations",
          data: props.links ? associations : [],
          getSourcePosition: (d) => d.from,
          getTargetPosition: (d) => d.to,
          getColor: [41, 113, 90, 190],
          getWidth: 1.5,
          parameters: { depthWriteEnabled: false },
        }),
      ],
    });
  }, [
    props.scenario,
    props.snapshot,
    props.coverage,
    props.buildings,
    props.links,
    loaded,
  ]);
  return (
    <div className={`map-stage mode-${props.mode}`}>
      <div ref={container} className="map-canvas" data-testid="network-map" />
      <div className="map-tag">
        <span className="live-dot" /> GEOGRAPHIC VIEW{" "}
        <span className="divider">/</span> <span>WGS 84</span>
      </div>
      <div className="map-tools">
        <button title="Zoom in" onClick={() => map.current?.zoomIn()}>
          <Plus size={18} />
        </button>
        <button title="Zoom out" onClick={() => map.current?.zoomOut()}>
          <Minus size={18} />
        </button>
        <hr />
        <button
          title="Recenter scenario"
          onClick={() =>
            map.current?.flyTo({ center: props.scenario.center, zoom: 15.5 })
          }
        >
          <LocateFixed size={18} />
        </button>
        <button
          title="Set study center to current map position"
          onClick={() => props.onCenter(position)}
        >
          <Crosshair size={18} />
        </button>
      </div>
      <div className="map-guide">
        <MapPin size={14} />
        {props.mode === "select"
          ? "Select an entity to inspect · drag to move"
          : props.mode === "move"
            ? "Click the map to move the selected entity"
            : `Click the map to deploy a ${props.mode === "ue" ? "UE" : "gNodeB"}`}
      </div>
      <div className="coverage-key">
        <div>
          <Layers size={14} />
          <strong>Estimated RSRP</strong>
          <span>50 m grid</span>
        </div>
        <div className="gradient" />
        <div className="key-values">
          <span>−120</span>
          <span>−100</span>
          <span>−80</span>
          <span>−60 dBm</span>
        </div>
        <p>Illustrative propagation · not a site survey</p>
      </div>
      <div className="map-coordinates">
        {position[1].toFixed(5)}° N &nbsp; {position[0].toFixed(5)}° E
      </div>
      {mapError && (
        <button className="map-warning" onClick={() => setMapError("")}>
          {mapError} ×
        </button>
      )}
    </div>
  );
}
