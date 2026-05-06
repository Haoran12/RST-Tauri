use std::mem;

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: String,
}

impl SseDecoder {
    pub fn push_str(&mut self, text: &str) -> Vec<String> {
        self.buffer.push_str(text);
        let mut events = Vec::new();

        while let Some((event, rest)) = split_next_event(&self.buffer) {
            events.extend(extract_data_events(event));
            self.buffer = rest.to_string();
        }

        events
    }

    pub fn finish(&mut self) -> Vec<String> {
        if self.buffer.trim().is_empty() {
            self.buffer.clear();
            return Vec::new();
        }

        let event = mem::take(&mut self.buffer);
        extract_data_events(&event)
    }
}

fn split_next_event(buffer: &str) -> Option<(&str, &str)> {
    let lf_lf = buffer.find("\n\n");
    let crlf_crlf = buffer.find("\r\n\r\n");

    match (lf_lf, crlf_crlf) {
        (Some(lf), Some(crlf)) if crlf <= lf => Some((&buffer[..crlf], &buffer[crlf + 4..])),
        (Some(lf), _) => Some((&buffer[..lf], &buffer[lf + 2..])),
        (None, Some(crlf)) => Some((&buffer[..crlf], &buffer[crlf + 4..])),
        (None, None) => None,
    }
}

fn extract_data_events(event: &str) -> Vec<String> {
    let data_lines = event
        .lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            line.strip_prefix("data:")
                .map(|data| data.strip_prefix(' ').unwrap_or(data).to_string())
        })
        .collect::<Vec<_>>();

    if data_lines.is_empty() {
        Vec::new()
    } else {
        vec![data_lines.join("\n")]
    }
}

#[cfg(test)]
mod tests {
    use super::SseDecoder;

    #[test]
    fn decodes_split_sse_events() {
        let mut decoder = SseDecoder::default();

        assert!(decoder.push_str("data: {\"a\"").is_empty());
        assert_eq!(decoder.push_str(":1}\n\n"), vec!["{\"a\":1}"]);
    }

    #[test]
    fn decodes_multiple_events_from_one_chunk() {
        let mut decoder = SseDecoder::default();

        assert_eq!(
            decoder.push_str("event: one\ndata: first\r\n\r\ndata: second\n\n"),
            vec!["first", "second"]
        );
    }

    #[test]
    fn joins_multiline_data_payloads() {
        let mut decoder = SseDecoder::default();

        assert_eq!(
            decoder.push_str("data: hello\ndata: world\n\n"),
            vec!["hello\nworld"]
        );
    }
}
