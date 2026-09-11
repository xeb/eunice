//! SSE framing over bytes: a network chunk is neither an event nor a UTF-8 boundary.
use anyhow::{bail, Result};

#[derive(Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    data: Vec<String>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > 32 * 1024 * 1024 {
            bail!("SSE event exceeds 32 MiB");
        }
        let mut events = Vec::new();
        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            let line = std::str::from_utf8(&self.buffer[..end])?
                .trim_end_matches('\r')
                .to_string();
            self.buffer.drain(..=end);
            if line.is_empty() {
                if !self.data.is_empty() {
                    events.push(self.data.join("\n"));
                    self.data.clear();
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data
                    .push(data.strip_prefix(' ').unwrap_or(data).to_string());
            }
        }
        Ok(events)
    }

    pub fn finish(&self) -> Result<()> {
        if !self.buffer.is_empty() || !self.data.is_empty() {
            bail!("Truncated SSE event");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_byte_boundary_preserves_utf8_and_multiline_data() {
        let input = ": heartbeat\r\nevent: text\r\ndata: {\"text\":\"hi 🦀\",\r\ndata: \"ok\":true}\r\n\r\ndata: end\n\n".as_bytes();
        for split in 0..=input.len() {
            let mut decoder = Decoder::default();
            let mut output = decoder.push(&input[..split]).unwrap();
            output.extend(decoder.push(&input[split..]).unwrap());
            decoder.finish().unwrap();
            assert_eq!(output, ["{\"text\":\"hi 🦀\",\n\"ok\":true}", "end"]);
        }
    }
    #[test]
    fn incomplete_events_are_errors() {
        for input in [b"data: {}".as_slice(), b"data: {}\n".as_slice()] {
            let mut decoder = Decoder::default();
            assert!(decoder.push(input).unwrap().is_empty());
            assert!(decoder.finish().is_err());
        }
    }
}
