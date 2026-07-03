pub struct Config {
    pub tab_width: usize,
}

const DEF_TAB_WIDTH: usize = 8;

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_width: DEF_TAB_WIDTH,
        }
    }
}
