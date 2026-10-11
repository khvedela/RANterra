# Geographic data attribution

© [OpenStreetMap contributors](https://www.openstreetmap.org/copyright). OSM data is available under the [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/).

`public/paris-buildings.json` and the buildings in `examples/paris.json` were extracted on **2026-10-11** from the OSM API map endpoint:

`https://api.openstreetmap.org/api/0.6/map?bbox=2.359,48.865,2.369,48.870`

The bounding box covers part of the République neighborhood in Paris. The snapshot contains 467 tagged building ways with all referenced nodes available, closed rings, 4–100 vertices per ring, and 6,298 total vertices. Other feature types were discarded. Coordinates were preserved. No building geometry was invented or generated. This is a partial extract, not complete geometry for the full RF grid, and not a measured radio environment. Dataset licensing is separate from the MIT application code license.

The app also requests OpenStreetMap Standard raster tiles and optional Nominatim/Overpass data. Attribution stays visible on the map. Public services provide no availability guarantee; local simulation does not depend on their continued availability once assets and geometry have loaded.
