/// A cursor over the comma-separated fields of an NMEA sentence body.
///
/// Fields are yielded left to right. An empty field (two adjacent commas)
/// yields `Some("")`, distinct from the `None` returned once the sentence
/// is exhausted, so a parser can tell an omitted trailing field from a
/// present-but-empty one.
#[derive(Clone, Debug)]
pub struct Fields<'a> {
    remainder: Option<&'a str>,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(body: &'a str) -> Self {
        Self {
            remainder: (!body.is_empty()).then_some(body),
        }
    }

    /// The next raw field, or `None` when no fields remain.
    pub fn next_field(&mut self) -> Option<&'a str> {
        let remainder = self.remainder?;
        match remainder.split_once(',') {
            Some((head, tail)) => {
                self.remainder = Some(tail);
                Some(head)
            }
            None => {
                self.remainder = None;
                Some(remainder)
            }
        }
    }
}

impl<'a> Iterator for Fields<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_field()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_body_has_no_fields() {
        assert_eq!(Fields::new("").next_field(), None);
    }

    #[test]
    fn yields_empty_fields_before_exhaustion() {
        // "0,,0,,," -> six fields, several of them empty.
        let fields: Vec<_> = Fields::new("0,,0,,,").collect();
        assert_eq!(fields, ["0", "", "0", "", "", ""]);
    }

    #[test]
    fn single_field() {
        let fields: Vec<_> = Fields::new("4395").collect();
        assert_eq!(fields, ["4395"]);
    }
}
