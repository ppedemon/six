# Six
[![Build](https://github.com/ppedemon/six/actions/workflows/ci.yml/badge.svg)](https://github.com/ppedemon/six/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

Six is just another vi clone. Certainly nothing that the world needed or expected.

## Why
I'm learning Rust, so I figured it would be nice to use the language for implementing
something non-trivial. A modal editor seems to be a reasonable undertaking.

## How
Nothing too fancy. I'm using the following crates:

  * [ropey](https://crates.io/crates/ropey) for backing the text buffers
  * [unicode-segmentation](https://crates.io/crates/unicode-segmentation) and [unicode-width](https://crates.io/crates/unicode-width) for dealing with unicode text
  * [nom](https://crates.io/crates/nom) for parsing ex commands

---
Crafted by ppedemon (☹️🍅)
