use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;

use anyhow::Context;
use crate::download::manifest::{BDPMFile, Encoding};

pub struct TabParser {
    reader: BufReader<Cursor<Vec<u8>>>,
    line_number: usize,
    buffer: Vec<String>,
    in_multiline: bool,
}

impl TabParser {
    /// Open a BDPM file, decode with the correct encoding, return a streaming parser.
    pub fn from_path(path: &Path, file: BDPMFile) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let encoding = match file.schema().encoding {
            Encoding::Windows1252 => encoding_rs::WINDOWS_1252,
            Encoding::Latin1 => encoding_rs::WINDOWS_1252, // ISO-8859-1 not in encoding_rs; practical difference only affects C1 controls which don't appear in BDPM
            Encoding::Utf8 => encoding_rs::UTF_8,
        };

        let (decoded, _, _) = encoding.decode(&bytes);
        let content = decoded.into_owned().into_bytes();

        let reader = BufReader::with_capacity(1 << 16, Cursor::new(content));
        Ok(Self {
            reader,
            line_number: 0,
            buffer: Vec::new(),
            in_multiline: false,
        })
    }
}

impl Iterator for TabParser {
    type Item = anyhow::Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        loop {
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    // Emit final buffered record if any
                    if !self.buffer.is_empty() {
                        let result = std::mem::take(&mut self.buffer);
                        return Some(Ok(decode_line(&result.join("\t"))));
                    }
                    return None; // EOF
                }
                Ok(_) => {}
                Err(e) => return Some(Err(anyhow::anyhow!("IO error: {}", e))),
            }

            self.line_number += 1;

            // Strip trailing \r from CRLF and trim
            let trimmed = line.trim_end_matches('\r').trim_end();

            // Skip empty lines
            if trimmed.is_empty() {
                line.clear();
                continue;
            }

            // Multi-line record handling: if line starts with 8-digit CIS, it's a new record.
            // Otherwise append to previous record's last field (avis field continuation).
            if is_cis_code(trimmed) {
                // Emit previous buffer if any
                if !self.buffer.is_empty() {
                    let result = std::mem::take(&mut self.buffer);
                    // Push current line first (so we don't lose it), then return old buffer
                    self.buffer.push(trimmed.to_string());
                    self.in_multiline = true;
                    line.clear();
                    return Some(Ok(decode_line(&result.join("\t"))));
                }
                self.buffer.push(trimmed.to_string());
                self.in_multiline = true;
                line.clear();
                continue;
            } else if self.in_multiline {
                // Continuation line — append to last field
                if let Some(last) = self.buffer.last_mut() {
                    last.push(' ');
                    last.push_str(trimmed);
                }
                line.clear();
                continue;
            } else {
                // Single-line record
                return Some(Ok(decode_line(trimmed)));
            }
        }
    }
}

/// Check if a line starts with an 8-digit CIS code
fn is_cis_code(line: &str) -> bool {
    line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        && line.len() >= 8
        && line.chars().take(8).all(|c| c.is_ascii_digit())
}

/// Split a line on tab characters.
fn decode_line(line: &str) -> Vec<String> {
    line.split('\t').map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cis_lines_emitted_no_loss() {
        // CIS_CIP_bdpm has only CIS-starting lines; every line triggers is_cis_code.
        // Old bug: even lines were discarded. Fix: push current, emit previous.
        let content: Vec<u8> = (1..=100u32)
            .map(|i| format!("{i:08}\tfield1\tfield2\n"))
            .collect::<String>()
            .into_bytes();
        let cursor = Cursor::new(content);
        let reader = BufReader::with_capacity(1024, cursor);
        let mut parser = TabParser {
            reader,
            line_number: 0,
            buffer: Vec::new(),
            in_multiline: false,
        };
        let rows: Vec<_> = (&mut parser).collect();
        assert_eq!(rows.len(), 100, "All 100 lines should be emitted, not half");
    }

    #[test]
    fn last_line_emitted_at_eof() {
        // Old bug: last row stayed in buffer, never emitted at EOF.
        // Fix: emit buffered record when EOF is reached.
        let content = "60000001\ta\tb\n60000002\tc\td\n";
        let cursor = Cursor::new(content.as_bytes().to_vec());
        let reader = BufReader::with_capacity(1024, cursor);
        let mut parser = TabParser {
            reader,
            line_number: 0,
            buffer: Vec::new(),
            in_multiline: false,
        };
        let rows: Vec<_> = (&mut parser).collect();
        assert_eq!(rows.len(), 2, "Both lines including last should be emitted");
        assert_eq!(rows[0].as_ref().unwrap()[0], "60000001");
        assert_eq!(rows[1].as_ref().unwrap()[0], "60000002");
    }
}
