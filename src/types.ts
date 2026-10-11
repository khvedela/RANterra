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
export interface Snssai {
  sst: number;
  sd: number[] | null;
}
export interface SessionRequest {
  dnn: string;
  snssai: Snssai;
}
export interface SessionPolicy {
  request: SessionRequest;
  max_sessions: number;
}
export interface Recording {
  format: "ranterra-experiment";
  version: 1;
  engine_profile: string;
  scenario: Scenario;
  commands: unknown[];
  time_ms: number;
  started: boolean;
  state_digest: string;
}
export interface Ue {
  signaling?: {
    delay_ms: number;
    drop: string;
    count: number;
    reject_n2: boolean;
  };
  session?: SessionRequest;
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
  version: 1 | 2;
  session_policies?: SessionPolicy[];
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
  session_pending?: number[];
  time_ms: number;
  started: boolean;
  pending: number;
  ues: UeStatus[];
  gnodebs: Record<string, string>;
  amf: Record<string, string>;
  events: NetworkEvent[];
  coverage: Sample[];
  event_limit_reached: boolean;
  session_failures: Record<
    string,
    {
      domain: string;
      cause: number;
      reason: string;
      retry_at_ms: number | null;
    }
  >;
  sessions: {
    ue_id: number;
    id: number;
    ip: number[];
    active: boolean;
    dnn: string;
    sst: number;
    sd: number[] | null;
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
  | { type: "recording"; purpose: "export" | "save"; revision: number }
  | { type: "compare"; recording: Recording; revision: number }
  | { type: "speed"; speed: number; revision: number }
  | { type: "load"; scenario: Scenario | Recording; revision: number }
  | { type: "play" | "pause" | "step"; revision: number };
export type WorkerResponse =
  | {
      type: "recording";
      recording: Recording;
      purpose: "export" | "save";
      revision: number;
    }
  | { type: "comparison"; snapshot: Snapshot; name: string; revision: number }
  | {
      type: "snapshot";
      scenario?: Scenario;
      snapshot: Snapshot;
      running: boolean;
      revision: number;
    }
  | { type: "error"; message: string; revision: number };
export type ToolMode = "select" | "gnodeb" | "ue" | "move" | "trajectory";
