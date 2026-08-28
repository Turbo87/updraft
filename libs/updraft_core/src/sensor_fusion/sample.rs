/// Whether an estimator accepted a sample for its time series.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleAcceptance {
    Accepted,
    Ignored,
}
