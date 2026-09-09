//! Describes a site's workout format as data the generator can consume.
//!
//! [`infer`] observes sample notation texts and emits the
//! [`fit_core::FormatSchema`] the parser reads its vocabulary from; see
//! [`infer`](mod@infer) for the evidence model. The swimdojo schema itself
//! lives in `fit_generator::parser::schema::swimdojo` (the parser's home);
//! [`swimdojo_schema`] re-exports it here as the inference baseline.

pub mod infer;

pub use fit_core::FormatSchema;
pub use infer::{Inference, infer, infer_texts};

/// The swimdojo notation schema (re-exported; owned by the generator).
pub fn swimdojo_schema() -> FormatSchema {
    fit_generator::parser::schema::swimdojo()
}
