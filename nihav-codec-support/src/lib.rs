//! Code and data for easier development of NihAV decoders.
#[allow(clippy::cast_lossless)]
#[allow(clippy::identity_op)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::unreadable_literal)]
pub mod codecs;

#[cfg(feature="dsp")]
#[allow(clippy::excessive_precision)]
#[allow(clippy::identity_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::unreadable_literal)]
pub mod dsp;

pub mod data;

pub mod test;

extern crate nihav_core;
