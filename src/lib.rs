//! claude-code-launcher library surface.
//!
//! Currently exposes the pluggable animation system (`anim`) used by the
//! prompt card's "little guy" and any interchangeable replacements.

#![allow(dead_code)] // foundation API surface; some hooks land with main.rs integration

pub mod anim;
