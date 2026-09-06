export type AirspaceLimit =
  { unlimited: true } | { referenceDatum: 0 | 1 | 2; unit: 0 | 1 | 6; value: number };

export type AirspaceFrequency = {
  name?: string;
  primary?: boolean;
  remarks?: string;
  unit: 2;
  value: string;
};

export type AirspaceTransponderSetting = {
  code: string;
  primary: boolean;
  remarks?: string;
};

export type AirspaceOperatingPeriod = {
  byNotam: boolean;
  dayOfWeek: number;
  publicHolidaysExcluded: boolean;
  remarks?: string;
  startTime?: string;
  endTime?: string;
  sunrise: boolean;
  sunset: boolean;
};

export type AirspaceProperties = {
  id: string;
  sourceName: string;
  type: number;
  icaoClass: number;
  lowerLimit: AirspaceLimit;
  upperLimit: AirspaceLimit;
  name?: string;
  activity?: number;
  onDemand?: boolean;
  onRequest?: boolean;
  byNotam?: boolean;
  specialAgreement?: boolean;
  requestCompliance?: boolean;
  country?: string | string[];
  frequencies?: AirspaceFrequency[];
  transponderSettings?: AirspaceTransponderSetting[];
  hoursOfOperation?: {
    operatingHours: AirspaceOperatingPeriod[];
    remarks?: string;
  };
  activeFrom?: string;
  activeUntil?: string;
  remarks?: string;
  lowerLimitMin?: AirspaceLimit;
  upperLimitMax?: AirspaceLimit;
};
