# Decision Records

Significant technology and architecture decisions are recorded here as
lightweight ADRs (architecture decision records), numbered in the order they
were made. Each record captures the context at the time, the decision, the
alternatives that were considered and why they were discarded, and the
consequences we accepted.

The [design documents](../design/README.md) always describe the *current
target design* and are updated when a decision changes them; the records
here are append-only history and are not rewritten when circumstances
change. Superseding a decision means adding a new record and marking the
old one as superseded.

Supporting research (library comparisons, benchmarks, upstream code
studies) lives in [../research/](../research/) and is linked from the
records rather than duplicated in them.

## Index

- [0001](0001-svelte-maplibre-gl.md): use svelte-maplibre-gl to integrate
  MapLibre GL JS into the Svelte 5 frontend
