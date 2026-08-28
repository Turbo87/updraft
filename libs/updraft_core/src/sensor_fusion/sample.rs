/// Whether an estimator accepted a sample for its time series.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleAcceptance {
    Accepted,
    Ignored,
}

/// Identifies an altitude time series with its own sampling clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeDomain {
    Pressure,
    Gnss,
}
