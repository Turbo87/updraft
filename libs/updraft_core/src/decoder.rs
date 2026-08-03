use crate::time::Timestamp;
use std::collections::VecDeque;
use updraft_nmea::{Message, Step, parse};

/// Owns the byte buffer for one connection and drains framed sentences.
///
/// Framing, checksum validation and resynchronisation all live in
/// [`updraft_nmea::parse`].
///
/// The timestamp segments cover the buffered bytes in input order. Their
/// remaining byte counts always add up to the buffer length.
#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    timestamp_segments: VecDeque<TimestampSegment>,
}

/// Tracks the unconsumed bytes from one call to [`Decoder::push()`].
#[derive(Debug)]
struct TimestampSegment {
    remaining_bytes: usize,
    ingested_at: Timestamp,
}

impl Decoder {
    /// Appends one byte input with its monotonic ingestion time.
    ///
    /// Call [`Decoder::next_message()`] until it returns `None` before calling
    /// this method again.
    pub fn push(&mut self, data: &[u8], ingested_at: Timestamp) {
        if data.is_empty() {
            return;
        }

        self.buffer.extend_from_slice(data);
        self.timestamp_segments.push_back(TimestampSegment {
            remaining_bytes: data.len(),
            ingested_at,
        });
    }

    /// Pulls the next complete sentence, discarding anything unframeable
    /// in front of it. Returns `None` when the buffer holds only a partial
    /// sentence. The returned timestamp belongs to the byte input that
    /// supplied the sentence's first byte.
    ///
    /// Named `next_message` rather than `next` because `Decoder` is not an
    /// iterator: it is refillable, and `None` means "not yet", not "done".
    pub fn next_message(&mut self) -> Option<(Message, Timestamp)> {
        loop {
            let ingested_at = self
                .timestamp_segments
                .front()
                .map(|segment| segment.ingested_at);
            let mut remaining: &[u8] = &self.buffer;
            let step = parse(&mut remaining);
            let consumed = self.buffer.len() - remaining.len();
            self.buffer.drain(..consumed);
            self.discard_timestamps(consumed);

            match step {
                Step::Incomplete => return None,
                Step::Frame(message) => {
                    let ingested_at =
                        ingested_at.expect("decoded sentence must have an ingestion timestamp");
                    return Some((message, ingested_at));
                }
                Step::Rejected(_) => {}
            }
        }
    }

    /// Keeps the timestamp segments aligned after the parser drains a prefix.
    fn discard_timestamps(&mut self, mut consumed_bytes: usize) {
        while let Some(segment) = self.timestamp_segments.front_mut() {
            if consumed_bytes < segment.remaining_bytes {
                segment.remaining_bytes -= consumed_bytes;
                consumed_bytes = 0;
                break;
            }

            consumed_bytes -= segment.remaining_bytes;
            self.timestamp_segments.pop_front();
        }

        debug_assert_eq!(consumed_bytes, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use std::assert_matches;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const GGA: &[u8] = b"$GPGGA,120000.00,5049.38,N,00611.16,E,1,08,1.0,200.0,M,47.0,M,,\r\n";

    #[test]
    fn decodes_a_sentence_split_across_two_pushes() {
        let mut decoder = Decoder::default();

        decoder.push(
            b"$GPRMC,120000.00,A,5049.38,N,0",
            Timestamp::from_millis(1_000),
        );
        assert_none!(decoder.next_message());

        decoder.push(
            b"0611.16,E,45.0,270.0,010126,,,A\r\n",
            Timestamp::from_millis(2_000),
        );
        let (message, ingested_at) = assert_some!(decoder.next_message());

        assert_matches!(message, Message::Rmc(_));
        assert_eq!(ingested_at, Timestamp::from_millis(1_000));
    }

    #[test]
    fn timestamps_each_sentence_from_the_input_that_started_it() {
        let mut decoder = Decoder::default();
        decoder.push(
            b"$GPRMC,120000.00,A,5049.38,N,0",
            Timestamp::from_millis(1_000),
        );
        assert_none!(decoder.next_message());

        decoder.push(
            &[b"0611.16,E,45.0,270.0,010126,,,A\r\n".as_slice(), GGA].concat(),
            Timestamp::from_millis(2_000),
        );
        let (first_message, first_ingested_at) = assert_some!(decoder.next_message());
        let (second_message, second_ingested_at) = assert_some!(decoder.next_message());

        assert_matches!(first_message, Message::Rmc(_));
        assert_eq!(first_ingested_at, Timestamp::from_millis(1_000));
        assert_matches!(second_message, Message::Gga(_));
        assert_eq!(second_ingested_at, Timestamp::from_millis(2_000));
    }

    #[test]
    fn drains_every_buffered_sentence_before_returning_none() {
        let mut decoder = Decoder::default();
        decoder.push(&[GGA, RMC].concat(), Timestamp::default());

        assert_matches!(decoder.next_message(), Some((Message::Gga(_), _)));
        assert_matches!(decoder.next_message(), Some((Message::Rmc(_), _)));
        assert_none!(decoder.next_message());
    }

    #[test]
    fn recovers_after_leading_noise() {
        let mut decoder = Decoder::default();
        decoder.push(
            &[b"garbage bytes\r\n".as_slice(), RMC].concat(),
            Timestamp::default(),
        );

        assert_matches!(decoder.next_message(), Some((Message::Rmc(_), _)));
    }
}
