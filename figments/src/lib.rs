#![no_std]
#![doc = include_str!("../../README.md")]
#![warn(missing_docs)]
#![feature(step_trait)]
pub mod pixbuf;
pub mod geometry;
pub mod mappings;
pub mod render;
pub mod liber8tion;
pub mod pixels;
pub mod prelude;

#[cfg(feature="alloc")]
pub mod surface;
#[cfg(feature="alloc")]
extern crate alloc;
#[cfg(feature="alloc")]
mod atomics;