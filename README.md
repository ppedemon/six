# Six
Six is just another vi clone. Certainly nothing that the world needed or expected.

## Why
I'm learning Rust, so I figured it would be nice to use the language for implementing
something non-trivial. A modal editor seems to be a reasonable undertaking.

## How
Nothing too fancy. I'm using the following crates:

  * [ropey](https://crates.io/crates/ropey) for backing the text buffers
  * [hecs](https://crates.io/crates/hecs) for implementing the editor's state while keeping the borrow-checker happy
  * [nom](https://crates.io/crates/nom) for parsing ex commands
 
---
Crafted by ppedemon (☹️🍅)