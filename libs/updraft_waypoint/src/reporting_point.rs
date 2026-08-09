/// One VFR reporting point.
#[derive(Clone, Debug, PartialEq)]
pub struct ReportingPoint {
    /// Whether a report at this point is compulsory. CUP supplies no value.
    pub compulsory: Option<bool>,
    /// The unvalidated OpenAIP airport references that this point serves.
    pub airport_references: Vec<Box<str>>,
}
