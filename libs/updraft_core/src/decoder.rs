use updraft_nmea::{Message, Step, parse};

/// Owns the byte buffer for one connection and drains framed sentences.
///
/// Framing, checksum validation and resynchronisation all live in
/// [`updraft_nmea::parse`].
#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Pulls the next complete sentence, discarding anything unframeable
    /// in front of it. Returns `None` when the buffer holds only a partial
    /// sentence.
    ///
    /// Named `next_message` rather than `next` because `Decoder` is not an
    /// iterator: it is refillable, and `None` means "not yet", not "done".
    pub fn next_message(&mut self) -> Option<Message> {
        loop {
            let mut remaining: &[u8] = &self.buffer;
            let step = parse(&mut remaining);
            let consumed = self.buffer.len() - remaining.len();
            self.buffer.drain(..consumed);

            match step {
                Step::Incomplete => return None,
                Step::Frame(message) => return Some(message),
                Step::Rejected(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;
    use std::assert_matches;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const GGA: &[u8] = b"$GPGGA,120000.00,5049.38,N,00611.16,E,1,08,1.0,200.0,M,47.0,M,,\r\n";

    #[test]
    fn decodes_a_sentence_split_across_two_pushes() {
        let mut decoder = Decoder::default();

        decoder.push(b"$GPRMC,120000.00,A,5049.38,N,0");
        assert_none!(decoder.next_message());

        decoder.push(b"0611.16,E,45.0,270.0,010126,,,A\r\n");
        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
    }

    #[test]
    fn drains_every_buffered_sentence_before_returning_none() {
        let mut decoder = Decoder::default();
        decoder.push(&[GGA, RMC].concat());

        assert_matches!(decoder.next_message(), Some(Message::Gga(_)));
        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
        assert_none!(decoder.next_message());
    }

    #[test]
    fn recovers_after_leading_noise() {
        let mut decoder = Decoder::default();
        decoder.push(&[b"garbage bytes\r\n".as_slice(), RMC].concat());

        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
    }
}
