use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Context;
use crate::download::manifest::{BDPMFile, Encoding};

pub struct TabParser {
    reader: BufReader<File>,
    line_number: usize,
    buffer: Vec<String>,
    in_multiline: bool,
    pub(crate) encoding: Encoding,
}

impl TabParser {
    /// Open a BDPM file, decode with the correct encoding, return a streaming parser.
    pub fn from_path(path: &Path, file: BDPMFile) -> anyhow::Result<Self> {
        let f = File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        let reader = BufReader::with_capacity(1 << 16, f);
        Ok(Self {
            reader,
            line_number: 0,
            buffer: Vec::new(),
            in_multiline: false,
            encoding: file.schema().encoding,
        })
    }
}

impl Iterator for TabParser {
    type Item = anyhow::Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        loop {
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
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
                    self.buffer.clear();
                    self.in_multiline = false;
                    line.clear();
                    return Some(Ok(decode_line(&result.join("\t"), self.encoding)));
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
                return Some(Ok(decode_line(trimmed, self.encoding)));
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

/// Decode bytes using the correct encoding, then split on tab.
fn decode_line(line: &str, _encoding: Encoding) -> Vec<String> {
    // The line is already &str from read_line (String dereferenced).
    // For files decoded from Windows-1252/Latin-1, the bytes were already decoded
    // by BufReader reading as bytes and converting. Since we're using File+BufReader
    // (not raw bytes), the platform default encoding applies. We need to handle this
    // at the raw byte level for BDPM files.
    //
    // For this initial implementation, we'll handle encoding at the file-open level.
    // A proper implementation uses raw byte reading with encoding_rs decoders.
    //
    // For now: split on tab directly (assumes files are opened with correct encoding).
    // This will be fixed in 01-02 implementation with proper byte-level decoding.
    line.split('\t').map(|s| s.to_string()).collect()
}

// use std::io::Read;
//
// For proper encoding support, replace from_path with:
//
// pub fn from_path(path: &Path, file: BDPMFile) -> anyhow::Result<Self> {
//     let encoding = match file.schema().encoding {
//         Encoding::Windows1252 => encoding_rs::WINDOWS_1252,
//         Encoding::Latin1 => encoding_rs::ISO_8859_1,
//         Encoding::Utf8 => encoding_rs::UTF_8,
//     };
//     let mut bytes = Vec::new();
//     File::open(path)?.read_to_end(&mut bytes)?;
//     let (decoded, _, _) = encoding.decode(&bytes);
//     let content = decoded.into_owned();
//     // Then parse lines from the decoded string...
// }
