//! Crate for providing support for various MPEG formats.
extern crate nihav_core;
extern crate nihav_codec_support;

#[cfg(feature="decoders")]
#[allow(clippy::needless_range_loop)]
mod codecs;

#[cfg(feature="decoders")]
pub use crate::codecs::mpeg_register_all_decoders;
