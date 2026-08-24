use std::collections::{HashMap, hash_map::Entry};

use crate::cmd::EditOp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterData {
    Char { data: String },
    Line { data: String },
    Block { data: Vec<String> },
}

impl RegisterData {
    pub fn char(data: String) -> Self {
        Self::Char { data }
    }

    pub fn line(data: String) -> Self {
        Self::Line { data }
    }

    pub fn block(data: Vec<String>) -> Self {
        Self::Block { data }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RegisterData::Char { data } | RegisterData::Line { data } => data.is_empty(),
            RegisterData::Block { data } => data.is_empty(),
        }
    }

    pub fn append(&mut self, incoming: RegisterData) {
        let current = std::mem::replace(
            self,
            Self::Char {
                data: String::new(),
            },
        );

        *self = match (current, incoming) {
            (Self::Char { mut data }, Self::Char { data: other }) => {
                data.push_str(&other);
                Self::Char { data }
            }
            (Self::Char { data }, Self::Line { data: other })
            | (Self::Line { data }, Self::Char { data: other })
            | (Self::Line { data }, Self::Line { data: other }) => Self::Line {
                data: append(data, other),
            },
            (Self::Block { data }, Self::Block { data: other }) => Self::Line {
                data: append(vstack(data), vstack(other)),
            },
            (Self::Block { data }, Self::Line { data: other })
            | (Self::Block { data }, Self::Char { data: other }) => Self::Line {
                data: append(vstack(data), other),
            },
            (Self::Line { data }, Self::Block { data: other })
            | (Self::Char { data }, Self::Block { data: other }) => Self::Line {
                data: append(data, vstack(other)),
            },
        };
    }
}

fn vstack(data: impl IntoIterator<Item = String>) -> String {
    let mut s = String::new();
    for (i, r) in data.into_iter().enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str(&r);
    }
    s
}

fn append(mut left: String, right: String) -> String {
    if left.len() > 0 {
        ensure_nl(&mut left);
    }
    left.push_str(&right);
    ensure_nl(&mut left);
    left
}

fn ensure_nl(s: &mut String) {
    let len = s.len();
    if len == 0 || !s.ends_with('\n') {
        s.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append() {
        let mut left = "Hello".to_owned();
        let right = "World".to_owned();
        let result = append(left, right.clone());
        assert_eq!(result, "Hello\nWorld\n".to_owned());

        left = "Hello\n".to_owned();
        let result = append(left, right.clone());
        assert_eq!(result, "Hello\nWorld\n".to_owned());

        left = "".to_owned();
        let result = append(left, right.clone());
        assert_eq!(result, "World\n".to_owned());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    Unnamed,
    Named(char),
    Append(char),
    Numbered(u8),
}

impl Register {
    pub const SMALL_DELETE: Self = Self::Named('-');
    pub const BLACKHOLE: Self = Self::Named('_');
    pub const LAST_INSERT: Self = Self::Named('.');

    pub fn from(c: char) -> Self {
        match c {
            c if c.is_ascii_digit() => Self::Numbered((c as u8) - b'0'),
            c if c.is_ascii_whitespace() => Self::Append(c.to_ascii_lowercase()),
            c if c == '"' => Self::Unnamed,
            _ => Self::Named(c),
        }
    }

    pub fn is_readonly(&self) -> bool {
        match self {
            Self::Named(c) if "%#.:/=_".contains(*c) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct Registers {
    unnamed: Option<RegisterData>,
    named: HashMap<char, RegisterData>,
    numbered: [Option<RegisterData>; Self::NUM_REGS],
    last_insert: Vec<EditOp>,
}

impl Registers {
    const NUM_REGS: usize = 10;

    pub fn empty() -> Self {
        Self {
            unnamed: None,
            named: HashMap::new(),
            numbered: Default::default(),
            last_insert: Vec::new(),
        }
    }

    pub fn commit_insert_log(&mut self, ops: Vec<EditOp>) {
        self.last_insert = ops;
    }

    pub fn record_delete(&mut self, data: RegisterData) {
        for i in (2..Self::NUM_REGS).rev() {
            let data = std::mem::take(&mut self.numbered[i as usize - 1]);
            self.numbered[i as usize] = data;
        }
        self.numbered[1] = Some(data);
    }

    pub fn write(&mut self, reg: Register, data: RegisterData) {
        match reg {
            Register::Unnamed => self.unnamed = Some(data),
            Register::Named(c) => {
                self.named.insert(c, data);
            }
            Register::Append(c) => {
                let key = c.to_ascii_lowercase();
                match self.named.entry(key) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().append(data);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(data);
                    }
                }
            }
            Register::Numbered(i) => {
                if (i as usize) < Self::NUM_REGS {
                    self.numbered[i as usize] = Some(data);
                }
            }
        }
    }

    pub fn read(&self, reg: Register) -> Option<&RegisterData> {
        match reg {
            Register::Unnamed => self.unnamed.as_ref(),
            Register::Named(c) | Register::Append(c) => self.named.get(&c),
            Register::Numbered(i) => self.numbered.get(i as usize).and_then(Option::as_ref),
        }
    }

    pub fn last_insert(&self) -> &[EditOp] {
        &self.last_insert
    }

    pub fn record_small_delete(&mut self, reg: Option<char>, deleted: String) {
        match reg {
            None => {
                let data = RegisterData::char(deleted);
                self.write(Register::Unnamed, data.clone());
                self.write(Register::SMALL_DELETE, data);
            }
            Some(r) => {
                let r = Register::from(r);
                if r.is_readonly() {
                    return;
                }

                let data = RegisterData::char(deleted);
                self.record_delete(data.clone());
                self.write(Register::Unnamed, data.clone());
                self.write(r, data);
            }
        }
    }

    pub fn record_yank(&mut self, reg: Option<char>, data: RegisterData) {
        match reg {
            None | Some('"') => {
                self.write(Register::Unnamed, data.clone());
                self.write(Register::Numbered(0), data);
            }
            Some(r) => {
                let r = Register::from(r);
                if r.is_readonly() || r == Register::SMALL_DELETE {
                    return;
                }

                self.write(Register::Unnamed, data.clone());
                self.write(r, data);
            }
        }
    }
}
