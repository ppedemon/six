use std::{fmt, ops::Range};

use ropey::Rope;

// -- Undo Chunk ---

// An undo chunk us a basic unit of text editing work. It can be either
// a series of contiguous insertions or deletions.
pub enum UndoChunk {
    Insert {
        char_idx: usize,
        text: String,
        len_chars: usize,
        num_lines: usize,
    },
    Delete {
        char_idx: usize,
        text: String,
        len_chars: usize,
        num_lines: usize,
    },
}

impl fmt::Debug for UndoChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, char_idx, text) = match self {
            Self::Insert { char_idx, text, .. } => ("Insert", char_idx, text),
            Self::Delete { char_idx, text, .. } => ("Delete", char_idx, text),
        };

        f.debug_struct(kind)
            .field("char_idx", char_idx)
            .field("text", text)
            .finish()
    }
}

impl fmt::Display for UndoChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// -- Undo Transaction ---

// Undo transacton metadata. we will use this for computing Damage regions
// and status event messages.
pub struct UndoTxnMeta {
    pub timestamp_ms: u64,
    pub start_idx: usize,
    pub end_idx: usize,
    pub multi_line: bool,
}

impl UndoTxnMeta {
    fn new(timestamp_ms: u64, start_idx: usize, end_idx: usize, multi_line: bool) -> Self {
        Self {
            timestamp_ms,
            start_idx,
            end_idx,
            multi_line,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.timestamp_ms == u64::MAX && self.start_idx == usize::MAX && self.end_idx == 0
    }

    pub fn merge(&mut self, other: Self) {
        self.timestamp_ms = self.timestamp_ms.min(other.timestamp_ms);
        self.start_idx = self.start_idx.min(other.start_idx);
        self.end_idx = self.end_idx.max(other.end_idx);
        self.multi_line |= other.multi_line;
    }
}

impl Default for UndoTxnMeta {
    fn default() -> Self {
        Self {
            timestamp_ms: u64::MAX,
            start_idx: usize::MAX,
            end_idx: 0,
            multi_line: false,
        }
    }
}

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

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn range(&self) -> UndoTxnMeta {
        let mut start_idx = usize::MAX;
        let mut end_idx = 0;
        let mut multi_line = false;

        for chunk in &self.data {
            match chunk {
                UndoChunk::Insert {
                    char_idx,
                    text,
                    len_chars,
                    num_lines,
                } => {
                    start_idx = start_idx.min(*char_idx);
                    end_idx = end_idx.max(char_idx + len_chars);
                    multi_line |= *num_lines > 1;
                }
                UndoChunk::Delete {
                    char_idx,
                    text,
                    len_chars,
                    num_lines,
                } => {
                    start_idx = start_idx.min(*char_idx);
                    end_idx = end_idx.max(char_idx + len_chars);
                    multi_line |= *num_lines > 1;
                }
            }
        }

        UndoTxnMeta::new(self.timestamp_ms, start_idx, end_idx, multi_line)
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

// -- Undo Stack ---

// Stack of undo transactions. The chunk and transactions managing is
// taken care of by the UndoTxnAgg struct.
pub struct UndoStack {
    agg: UndoTxnAgg,
    txns: Vec<UndoTxn>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            agg: UndoTxnAgg::new(),
            txns: Vec::default(),
        }
    }

    pub fn pop(&mut self) -> Option<UndoTxn> {
        self.txns.pop()
    }

    pub fn commit(&mut self) {
        if let Some(txn) = self.agg.commit() {
            self.txns.push(txn);
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
                    self.chunks.push(UndoChunk::Insert {
                        char_idx: self.start_idx,
                        text: self.buffer.to_string(),
                        len_chars: self.buffer.len_chars(),
                        num_lines: self.buffer.len_lines(),
                    });
                }
                AggMode::Delete => {
                    self.chunks.push(UndoChunk::Delete {
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
