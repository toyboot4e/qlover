//! A stenography engine in Rust.
//!
//! Throughout this library, we denote time complexity of functions with [big O notation].
//!
//! [big O notation]: https://en.wikipedia.org/wiki/Big_O_notation
//! [Plover]: https://github.com/openstenoproject/plover

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod dictionary;
pub mod engine;
pub mod output;
pub mod protocol;
pub mod stroke;
pub mod translation;
pub mod util;
