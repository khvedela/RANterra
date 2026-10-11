import type { Scenario } from "./types";
export const demo: Scenario = {
  version: 1,
  name: "République · urban lab",
  seed: 42,
  center: [2.3639, 48.8674],
  gnodebs: [
    {
      id: 1,
      name: "République North",
      lon: 2.3622,
      lat: 48.8688,
      frequency_mhz: 3500,
      tx_power_dbm: 46,
      height_m: 25,
      gain_dbi: 16,
    },
    {
      id: 2,
      name: "Temple South",
      lon: 2.3662,
      lat: 48.8658,
      frequency_mhz: 3500,
      tx_power_dbm: 43,
      height_m: 20,
      gain_dbi: 14,
    },
  ],
  ues: [
    { id: 101, name: "UE-01", lon: 2.3632, lat: 48.8682 },
    { id: 102, name: "UE-02", lon: 2.3657, lat: 48.8664 },
    { id: 103, name: "UE-03", lon: 2.3638, lat: 48.8659 },
    { id: 104, name: "UE-04", lon: 2.369, lat: 48.8669 },
  ],
  buildings: [],
};
