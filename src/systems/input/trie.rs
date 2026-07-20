#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindResult<V> {
    Hit(V),
    Miss,
    Partial,
}

pub struct Trie<K, V> {
    children: Vec<(K, Box<Trie<K, V>>)>,
    value: Option<V>,
}

impl<K, V> Trie<K, V>
where
    K: Clone + Eq,
{
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            value: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    // Return true if after insertion trie has a prefix
    pub fn insert(&mut self, xs: &[K], v: V) -> bool {
        let mut node = self;
        let mut saw_prefix = false;

        for x in xs {
            saw_prefix |= node.value.is_some();

            let idx = match node.children.iter().position(|(k, _)| k == x) {
                Some(i) => i,
                None => {
                    node.children.push((x.clone(), Box::new(Trie::new())));
                    node.children.len() - 1
                }
            };

            node = node.children[idx].1.as_mut();
        }

        node.value = Some(v);
        saw_prefix || !node.is_empty()
    }

    fn node(&self, xs: &[K]) -> Option<&Trie<K, V>> {
        let mut node = self;
        for x in xs {
            match node.children.iter().find(|(k, _)| k == x) {
                None => return None,
                Some((_, n)) => node = n,
            }
        }
        Some(node)
    }

    pub fn find(&self, s: &[K]) -> FindResult<&V> {
        match self.node(s) {
            None => FindResult::Miss,
            Some(node) if node.value.is_some() => FindResult::Hit(node.value.as_ref().unwrap()),
            Some(_) => FindResult::Partial,
        }
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn test_insert() {
        let mut trie = Trie::new();
        let has_prefix = trie.insert("dd".as_bytes(), ());
        assert!(!has_prefix);

        let result = trie.find("dd".as_bytes());
        assert_eq!(result, FindResult::Hit(&()))
    }

    #[test]
    fn test_prefix_insert() {
        let mut trie = Trie::new();

        let mut has_prefix = trie.insert("dd".as_bytes(), ());
        assert!(!has_prefix);

        has_prefix = trie.insert("de".as_bytes(), ());
        assert!(!has_prefix);

        has_prefix = trie.insert("d".as_bytes(), ());
        assert!(has_prefix);

        has_prefix = trie.insert("ddg".as_bytes(), ());
        assert!(has_prefix);

        has_prefix = trie.insert("c".as_bytes(), ());
        assert!(!has_prefix);
    }

    #[test]
    fn test_big_prefix() {
        let mut trie = Trie::new();

        let mut has_prefix = trie.insert("a".as_bytes(), ());
        assert!(!has_prefix);

        has_prefix = trie.insert("abc".as_bytes(), ());
        assert!(has_prefix);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Op {
        GotoLine,
        Del,
        Exit,
        Save,
        PgUp,
        Split,
    }

    #[test]
    fn test_find() {
        let words = vec![
            ("gg", Op::GotoLine),
            ("G", Op::GotoLine),
            ("d", Op::Del),
            ("ZQ", Op::Exit),
            ("ZZ", Op::Save),
        ];
        let mut trie = Trie::new();
        let mut has_prefix = false;

        for (k, v) in &words {
            has_prefix = has_prefix || trie.insert(k.as_bytes(), *v);
        }

        assert!(!has_prefix);

        for (k, v) in &words {
            assert_eq!(FindResult::Hit(v), trie.find(k.as_bytes()));
        }

        assert_eq!(FindResult::Miss, trie.find("banana".as_bytes()));
        assert_eq!(FindResult::Partial, trie.find("g".as_bytes()));
        assert_eq!(FindResult::Partial, trie.find("Z".as_bytes()));
    }

    #[test]
    fn test_key_events() {
        let cmds = vec![
            // gg
            (
                vec![
                    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()),
                    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()),
                ],
                Op::GotoLine,
            ),
            // ZZ
            (
                vec![
                    KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::empty()),
                    KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::empty()),
                ],
                Op::Save,
            ),
            // ZQ
            (
                vec![
                    KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::empty()),
                    KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::empty()),
                ],
                Op::Exit,
            ),
            // G
            (
                vec![KeyEvent::new(KeyCode::Char('G'), KeyModifiers::empty())],
                Op::GotoLine,
            ),
            // <C-u>
            (
                vec![KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)],
                Op::PgUp,
            ),
            // <C-w>s
            (
                vec![
                    KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                    KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()),
                ],
                Op::Split,
            ),
        ];

        let mut trie = Trie::new();
        let mut has_prefix = false;

        for (cmd, op) in &cmds {
            has_prefix = trie.insert(cmd, *op);
        }

        assert!(!has_prefix);

        for (cmd, op) in &cmds {
            assert_eq!(FindResult::Hit(op), trie.find(cmd));
        }

        assert_eq!(
            FindResult::Partial,
            trie.find(&vec![KeyEvent::new(
                KeyCode::Char('Z'),
                KeyModifiers::empty()
            )])
        );

        assert_eq!(
            FindResult::Miss,
            trie.find(&vec![KeyEvent::new(
                KeyCode::Char('d'),
                KeyModifiers::empty()
            )])
        );
    }
}
