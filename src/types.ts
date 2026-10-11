export type Point = [number, number];
export interface Gnodeb {
  id: number;
  name: string;
  lon: number;
  lat: number;
  frequency_mhz: number;
  tx_power_dbm: number;
  height_m: number;
  gain_dbi: number;
  antenna?: {
    sector: boolean;
    azimuth_deg: number;
    downtilt_deg: number;
    resource_blocks: number;
    numerology: number;
  };
}
export interface Ue {
  fault?: string;
  mobility?: { waypoints: Point[]; speed_mps: number };
  id: number;
  name: string;
  lon: number;
  lat: number;
}
export interface Building {
  id: string;
  ring: Point[];
}
export interface Scenario {
  version: 1;
  radio?: {
    model: "Legacy" | "UMi" | "UMa";
    shadow_fading: boolean;
    coverage_dbm: number;
    hysteresis_db: number;
    time_to_trigger_ms: number;
    duration_ms: number;
  };
  name: string;
  seed: number;
  center: Point;
  gnodebs: Gnodeb[];
  ues: Ue[];
  buildings: Building[];
}
export interface UeStatus {
  id: number;
  state: string;
  serving_cell: number | null;
  rsrp: number | null;
  position: Point;
  sinr_db: number | null;
  failure: string | null;
  registration_latency_ms: number | null;
}
export interface NetworkEvent {
  id: number;
  time_ms: number;
  ue_id: number;
  source: string;
  destination: string;
  kind: string;
  detail: string;
  hex: string;
  layer: string;
}
export interface Sample {
  position: Point;
  rsrp: number;
  cell_id: number;
}
export interface Snapshot {
  time_ms: number;
  started: boolean;
  pending: number;
  ues: UeStatus[];
  gnodebs: Record<string, string>;
  amf: Record<string, string>;
  events: NetworkEvent[];
  coverage: Sample[];
  event_limit_reached: boolean;
  sessions: {
    ue_id: number;
    id: number;
    ip: number[];
    active: boolean;
    dnn: string;
    sst: number;
    sent: number;
    delivered: number;
    dropped: number;
    last_rtt_ms: number | null;
  }[];
}
export type WorkerCommand =
  | {
      type: "session";
      id: number;
      action: "establish" | "release" | "echo" | "deregister";
      loss: number;
      revision: number;
    }
  | { type: "speed"; speed: number; revision: number }
  | { type: "load"; scenario: Scenario; revision: number }
  | { type: "play" | "pause" | "step"; revision: number };
export type WorkerResponse =
  | { type: "snapshot"; snapshot: Snapshot; running: boolean; revision: number }
  | { type: "error"; message: string; revision: number };
export type ToolMode = "select" | "gnodeb" | "ue" | "move" | "trajectory";
