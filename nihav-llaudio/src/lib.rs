extern crate nihav_core;
extern crate nihav_codec_support;

#[allow(clippy::comparison_chain)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::verbose_bit_mask)]
mod codecs;
#[allow(clippy::unreadable_literal)]
mod demuxers;
mod muxers;
pub use crate::codecs::llaudio_register_all_decoders;
pub use crate::demuxers::llaudio_register_all_demuxers;
pub use crate::codecs::llaudio_register_all_encoders;
pub use crate::muxers::llaudio_register_all_muxers;
