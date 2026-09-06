import type * as GeoJSON from 'geojson';
import type { AirspaceProperties } from '$lib/airspace';

/** Fixed airspace GeoJSON for browser tests and Storybook. */
export const AIRSPACE_BROWSER_FIXTURE = {
  type: 'FeatureCollection',
  features: [
    {
      type: 'Feature',
      id: '1:0:0',
      properties: {
        id: '1:0:0',
        sourceName: 'browser-fixture.txt',
        activeFrom: '2026-04-12T08:30:00Z',
        activeUntil: '2026-04-12T17:45:00Z',
        activity: 5,
        byNotam: true,
        country: ['DE', 'AT'],
        frequencies: [
          {
            name: 'TOWER',
            primary: true,
            remarks: 'EMERGENCIES ONLY',
            unit: 2,
            value: '123.450',
          },
          {
            remarks: 'GUARD',
            unit: 2,
            value: '121.500',
          },
        ],
        hoursOfOperation: {
          operatingHours: [
            {
              byNotam: false,
              dayOfWeek: 6,
              publicHolidaysExcluded: true,
              remarks: 'DAYLIGHT HOURS',
              sunrise: true,
              sunset: true,
            },
          ],
          remarks: 'LOCAL TIME',
        },
        icaoClass: 3,
        lowerLimit: { referenceDatum: 0, unit: 0, value: 0 },
        lowerLimitMin: { referenceDatum: 0, unit: 1, value: 500 },
        name: 'Düsseldorf CTR',
        onDemand: true,
        onRequest: false,
        remarks: 'ACTIVE DURING GLIDER EVENTS',
        requestCompliance: true,
        specialAgreement: false,
        transponderSettings: [{ code: '0123', primary: true, remarks: 'WHEN ACTIVE' }],
        type: 4,
        upperLimit: { referenceDatum: 1, unit: 1, value: 5000 },
        upperLimitMax: { referenceDatum: 2, unit: 6, value: 120 },
      },
      geometry: {
        type: 'Polygon',
        coordinates: [
          [
            [6.16, 50.81],
            [6.185, 50.81],
            [6.185, 50.835],
            [6.16, 50.835],
            [6.16, 50.81],
          ],
        ],
      },
    },
    {
      type: 'Feature',
      id: '1:0:1',
      properties: {
        id: '1:0:1',
        sourceName: 'browser-fixture.txt',
        icaoClass: 4,
        lowerLimit: { referenceDatum: 0, unit: 0, value: 0 },
        name: 'Köln RMZ',
        type: 6,
        upperLimit: { unlimited: true },
      },
      geometry: {
        type: 'Polygon',
        coordinates: [
          [
            [6.18, 50.81],
            [6.212, 50.81],
            [6.212, 50.835],
            [6.18, 50.835],
            [6.18, 50.81],
          ],
        ],
      },
    },
  ],
} satisfies GeoJSON.FeatureCollection<GeoJSON.Polygon, AirspaceProperties>;
