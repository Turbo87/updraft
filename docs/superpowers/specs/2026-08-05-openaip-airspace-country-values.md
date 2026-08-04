# OpenAIP Airspace Country Values

## Purpose

The OpenAIP airspace schema permits one country value or an array of country
values. Updraft preserves these values without validating them against a
country-code registry. This keeps source extensions such as `XK` intact.

## Superseded specification sections

This specification supersedes only the country-code validation requirements in
the **Canonical airspace** and **Country and remarks** sections of
[`2026-08-05-openaip-airspace-model-design.md`](2026-08-05-openaip-airspace-model-design.md).

All other requirements in that specification remain active.

## Canonical model

Each `Airspace` has a `country_codes: Vec<Box<str>>` collection. The collection
stores raw source values. It does not validate membership, length, case, or
character content.

An OpenAIP scalar value produces one collection entry. An OpenAIP array
preserves its values in source order. An absent value produces an empty
collection. OpenAir also produces an empty collection because the current
parser does not expose country values.

Country codes are not part of the GeoJSON rendering subset.

## Testing

The OpenAir importer test verifies that imported airspaces have no country
codes. A future OpenAIP adapter test must verify scalar and array normalization.
It must also verify that unrecognized values remain unchanged.
