# Aircraft Profiles & Presets

A user can have multiple aircraft profiles, but only one active profile at a
time. A profile can be created from scratch or copied from a preset.

## Preset

A read-only catalogue entry describing an aircraft type and its variants (propulsion, build variants, wingspan).

A preset contains:

- Display name (ASG 29E)
- Base model (ASG 29)
- Polar coefficients and reference mass
- Empty and max-takeoff mass
- Ballast tank capacities
- Wingspan and area
- VNE and Stall speeds
- WeGlide ID and fallback handicap
- Flap speed ranges
- CG limits and arms
- Number of seats

Note that most of these fields are optional, and the preset may be incomplete. The user can override any of these fields in their profile.

## Profile

A profile is a specific aircraft with registration (and callsign), not an
aircraft type.

Compared to a preset, a profile may additionally contain:

- Registration (D-1234)
- Callsign (TH)
- a reference to a device config (see [devices.md](devices.md))

A user can override any of the preset fields in their aircraft profiles.

## The three-level hierarchy

A preset is a three-level tree: a *base model*, its *build/propulsion variants*, and each variant's *wingspan configurations*. A profile copies a single variant, so it pins one airframe (build variant and propulsion fixed) but may still carry *multiple wingspans* (e.g. removable tips).

The preset tree is shallow:

```
base model                    ASG 29
└─ build/propulsion variant   ASG 29 · ASG 29E · ASG 29 Es · ASG 29 RES
   └─ wingspan config         15 m · 18 m
```

The wingspan config is the leaf and carries most of the fields because they *vary by wingspan*.

Fixed-span gliders (LS 4, ASW 27) simply have one leaf. Don't force a span picker when there's only one. Spans are an arbitrary set, not just 15/18 (Ventus 16.6 m, ASH 31 21 m), so model the leaves as a list.

## Mass semantics

There are four distinct masses:

- **reference mass**: the mass the base polar was measured/computed at. Feeds the polar math and is stored on the leaf alongside the coefficients.
- **empty mass**: bare airframe per the weighing report. Informational and a W&B input, but *not* a polar input.
- **max-takeoff mass**: a **limit for overload warnings only**, never a computation input. Tag it as such so it's never mistaken for a physics value.
- **total mass**: (empty + crew + water) is what actually scales the polar, via `GlidePolar::with_total_mass`.

## Relation to `updraft_polar`

`updraft_polar` provides the polar *math* (`PolarCoefficients`, `GlidePolar`, and its `new` / `with_total_mass` / `with_bugs` operations), but owns no catalogue. The preset catalogue lives in a separate `updraft_aircraft_presets` crate, which may depend on `updraft_polar` for the coefficient types.

## Open questions

- **Where does the handicap live, and how fresh is it?** Handicaps may change yearly, so a value baked into a `const` bit-rots. Leading candidate: **fetch the aircraft details (incl. `dmst_index`) from the WeGlide API on startup or on profile activation, cache them, and fall back to a build-time hardcoded value**. This keeps the app correct online and merely stale offline.
