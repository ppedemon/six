use std::{collections::VecDeque, fmt, ops::Range};

use ropey::Rope;

// --- Undo Chunk ---

pub enum Action {
    Insert,
    Delete,
}

// An undo chunk us a basic unit of text editing work. It can be either
// a series of contiguous insertions or deletions.
pub struct UndoChunk {
    pub action: Action,
    pub char_idx: usize,
    pub text: String,
    pub len_chars: usize,
    pub num_lines: usize,
}

impl fmt::Debug for UndoChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action = match self.action {
            Action::Insert => "Insert",
            Action::Delete => "Delete",
        };

        f.debug_struct(action)
            .field("char_idx", &self.char_idx)
            .field("text", &self.text)
            .finish()
    }
}

impl fmt::Display for UndoChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// --- Undo Summary ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoScope {
    None,
    PatchRange { start_line: usize, end_line: usize },
    DestroyFrom { start_line: usize },
}

#[derive(Debug, Clone, Copy)]
pub struct UndoSummary {
    pub timestamp_ms: u64,
    pub start_idx: usize,
    pub scope: UndoScope,
}

impl UndoSummary {
    pub fn single_line(timestamp_ms: u64, start_idx: usize, line: usize) -> Self {
        Self {
            timestamp_ms,
            start_idx,
            scope: UndoScope::PatchRange {
                start_line: line,
                end_line: line,
            },
        }
    }

    pub fn destroy_from(timestamp_ms: u64, start_idx: usize, line: usize) -> Self {
        Self {
            timestamp_ms,
            start_idx,
            scope: UndoScope::DestroyFrom { start_line: line },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.scope == UndoScope::None
    }

    pub fn merge(&mut self, other: UndoSummary) {
        self.timestamp_ms = self.timestamp_ms.min(other.timestamp_ms);
        self.start_idx = self.start_idx.min(other.start_idx);

        self.scope = match (self.scope, other.scope) {
            (UndoScope::None, _) => other.scope,
            (_, UndoScope::None) => self.scope,
            (
                UndoScope::PatchRange {
                    start_line: s1,
                    end_line: e1,
                },
                UndoScope::PatchRange {
                    start_line: s2,
                    end_line: e2,
                },
            ) => UndoScope::PatchRange {
                start_line: s1.min(s2),
                end_line: e1.max(e2),
            },
            (
                UndoScope::DestroyFrom { start_line: s1 },
                UndoScope::PatchRange { start_line: s2, .. },
            ) => UndoScope::DestroyFrom {
                start_line: s1.min(s2),
            },
            (
                UndoScope::PatchRange { start_line: s1, .. },
                UndoScope::DestroyFrom { start_line: s2 },
            ) => UndoScope::DestroyFrom {
                start_line: s1.min(s2),
            },
            (
                UndoScope::DestroyFrom { start_line: s1 },
                UndoScope::DestroyFrom { start_line: s2 },
            ) => UndoScope::DestroyFrom {
                start_line: s1.min(s2),
            },
        }
    }
}

impl Default for UndoSummary {
    fn default() -> Self {
        Self {
            timestamp_ms: u64::MAX,
            start_idx: usize::MAX,
            scope: UndoScope::None,
        }
    }
}

// --- Undo Transaction ---

// Undo transactions are a logical grouping of one or more undo chunks.
// The lifespan of a transaction is managed by systems by calling the
// `begin_txn` and `commit` functions in the UndoTxnAgg struct.
pub struct UndoTxn {
    pub data: Vec<UndoChunk>,
    pub timestamp_ms: u64,
}

impl UndoTxn {
    pub fn new(data: Vec<UndoChunk>, timestamp_ms: u64) -> Self {
        Self { data, timestamp_ms }
    }
}

impl fmt::Debug for UndoTxn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoTxn")
            .field("timestamp_ms", &self.timestamp_ms)
            .field("chunk_count", &self.data.len())
            .field("data", &self.data)
            .finish()
    }
}

impl fmt::Display for UndoTxn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// --- Undo Stack ---

// Stack of undo transactions. The chunk and transactions managing is
// taken care of by the UndoTxnAgg struct.
pub struct UndoStack {
    agg: UndoTxnAgg,
    txns: VecDeque<UndoTxn>,
}

impl UndoStack {
    const MAX_TXNS: usize = 256;

    pub fn new() -> Self {
        Self {
            agg: UndoTxnAgg::new(),
            txns: VecDeque::with_capacity(Self::MAX_TXNS),
        }
    }

    pub fn pop(&mut self) -> Option<UndoTxn> {
        self.txns.pop_back()
    }

    pub fn commit(&mut self) {
        if let Some(txn) = self.agg.commit() {
            if self.txns.len() >= Self::MAX_TXNS {
                self.txns.pop_front();
            }
            self.txns.push_back(txn);
        }
    }

    pub fn insert_char(&mut self, char_idx: usize, c: char) {
        self.agg.insert_char(char_idx, c);
    }

    pub fn insert_str(&mut self, char_idx: usize, s: &str) {
        self.agg.insert_str(char_idx, s);
    }

    pub fn insert_rope(&mut self, char_idx: usize, rope: &Rope) {
        self.agg.insert_rope(char_idx, rope);
    }

    pub fn remove_char(&mut self, char_range: Range<usize>, c: char) {
        self.agg.remove_char(char_range, c);
    }

    pub fn remove(&mut self, char_range: Range<usize>, text: &str) {
        self.agg.remove(char_range, text);
    }
}

impl fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoStack")
            .field("txn_count", &self.txns.len())
            .field("txns", &self.txns)
            .finish()
    }
}

impl fmt::Display for UndoStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// --- Undo aggregator ---

#[derive(Debug)]
enum AggMode {
    None,
    Insert,
    Delete,
}

struct UndoTxnAgg {
    in_txn: bool,
    creation_ts: u64,
    mode: AggMode,
    start_idx: usize,
    end_idx: usize,
    buffer: Rope,
    chunks: Vec<UndoChunk>,
}

impl UndoTxnAgg {
    fn new() -> Self {
        Self {
            in_txn: false,
            creation_ts: 0,
            mode: AggMode::None,
            start_idx: 0,
            end_idx: 0,
            buffer: Rope::new(),
            chunks: Vec::with_capacity(8),
        }
    }

    fn begin_txn(&mut self) {
        if self.in_txn {
            return;
        }

        self.in_txn = true;
        self.creation_ts = Self::now();
        self.mode = AggMode::None;
        self.start_idx = 0;
        self.end_idx = 0;
        self.buffer.remove(0..self.buffer.len_chars());
        self.chunks.clear();
    }

    fn commit(&mut self) -> Option<UndoTxn> {
        if !self.in_txn {
            return None;
        }

        self.in_txn = false;
        self.flush();

        if self.chunks.is_empty() {
            None
        } else {
            let txn = UndoTxn::new(std::mem::take(&mut self.chunks), self.creation_ts);
            #[cfg(feature = "dev")]
            tracing::debug!("UndoTxnAgg::commit: {txn:?}");
            Some(txn)
        }
    }

    fn insert_char(&mut self, char_idx: usize, c: char) {
        self.insert_preamble(char_idx);
        self.buffer.insert_char(self.buffer.len_chars(), c);
        self.end_idx += 1;
    }

    fn insert_str(&mut self, char_idx: usize, s: &str) {
        self.insert_preamble(char_idx);
        self.buffer.insert(self.buffer.len_chars(), s);
        self.end_idx += s.chars().count();
    }

    fn insert_rope(&mut self, char_idx: usize, rope: &Rope) {
        self.insert_preamble(char_idx);
        self.buffer.append(rope.clone());
        self.end_idx += rope.len_chars();
    }

    fn insert_preamble(&mut self, char_idx: usize) {
        if !self.in_txn {
            self.begin_txn();
        }

        let needs_flush = match self.mode {
            AggMode::None => true,
            AggMode::Insert => char_idx != self.end_idx,
            AggMode::Delete => true,
        };

        if needs_flush {
            self.flush();
            self.start_idx = char_idx;
            self.end_idx = char_idx;
        }

        self.mode = AggMode::Insert;
    }

    fn remove_char(&mut self, char_range: Range<usize>, c: char) {
        self.remove_preamble(&char_range);

        // char_range.end == self.start_idx: contiguous backspace deletes
        // char_range.start == self.start_idx: contiguous deletes (via Del key)

        if char_range.end == self.start_idx {
            self.start_idx = char_range.start;
            self.buffer.insert_char(0, c);
        } else if char_range.start == self.start_idx {
            let last_idx = self.buffer.len_chars();
            self.end_idx += char_range.len();
            self.buffer.insert_char(last_idx, c);
        }
    }

    fn remove(&mut self, char_range: Range<usize>, text: &str) {
        self.remove_preamble(&char_range);

        if char_range.end == self.start_idx {
            self.start_idx = char_range.start;
            self.buffer.insert(0, text);
        } else if char_range.start == self.start_idx {
            let last_idx = self.buffer.len_chars();
            self.end_idx += char_range.len();
            self.buffer.insert(last_idx, text);
        }
    }

    fn remove_preamble(&mut self, char_range: &Range<usize>) {
        if !self.in_txn {
            self.begin_txn();
        }

        let needs_flush = match self.mode {
            AggMode::None => true,
            AggMode::Insert => true,
            AggMode::Delete => {
                char_range.start != self.start_idx && char_range.end != self.start_idx
            }
        };

        if needs_flush {
            self.flush();
            self.start_idx = char_range.start;
            self.end_idx = char_range.end;
        }

        self.mode = AggMode::Delete;
    }

    fn flush(&mut self) {
        if self.buffer.len_chars() > 0 {
            match self.mode {
                AggMode::Insert => {
                    self.chunks.push(UndoChunk {
                        action: Action::Insert,
                        char_idx: self.start_idx,
                        text: self.buffer.to_string(),
                        len_chars: self.buffer.len_chars(),
                        num_lines: self.buffer.len_lines(),
                    });
                }
                AggMode::Delete => {
                    self.chunks.push(UndoChunk {
                        action: Action::Delete,
                        char_idx: self.start_idx,
                        text: self.buffer.to_string(),
                        len_chars: self.buffer.len_chars(),
                        num_lines: self.buffer.len_lines(),
                    });
                }
                AggMode::None => {}
            }

            self.buffer.remove(0..self.buffer.len_chars());
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
