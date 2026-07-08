use std::{collections::HashMap, sync::LazyLock};

static DIGRAPHS: LazyLock<HashMap<(char, char), char>> = LazyLock::new(|| load_digraphs());

pub fn load_digraphs() -> HashMap<(char, char), char> {
    let digraphs = include_str!("../resources/digraph.csv");
    let mut map = HashMap::with_capacity(1400);

    for line in digraphs.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut toks = line.split(',');
        let c0_opt = toks.next().and_then(|c| c.chars().next());
        let c1_opt = toks.next().and_then(|c| c.chars().next());
        let dg_opt = toks
            .next()
            .map(|s| s.trim_start_matches("0x"))
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .and_then(char::from_u32);

        if let (Some(c0), Some(c1), Some(dg)) = (c0_opt, c1_opt, dg_opt) {
            map.insert((c0, c1), dg);
        }
    }

    map
}

pub fn get(c0: char, c1: char) -> Option<char> {
    DIGRAPHS.get(&(c0, c1)).copied()
}
