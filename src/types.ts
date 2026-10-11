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
}
export interface Ue {
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
}
export type WorkerCommand =
  | { type: "load"; scenario: Scenario; revision: number }
  | { type: "play" | "pause" | "step"; revision: number };
export type WorkerResponse =
  | { type: "snapshot"; snapshot: Snapshot; running: boolean; revision: number }
  | { type: "error"; message: string; revision: number };
export type ToolMode = "select" | "gnodeb" | "ue" | "move";
