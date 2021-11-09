use nihav_core::codecs::*;

macro_rules! validate {
    ($a:expr) => { if !$a { println!("check failed at {}:{}", file!(), line!()); return Err(DecoderError::InvalidData); } };
}

#[cfg(feature="decoder_aac")]
#[allow(clippy::manual_memcpy)]
#[allow(clippy::useless_let_if_seq)]
mod aac;
#[cfg(feature="decoder_mpa")]
#[allow(clippy::excessive_precision)]
mod mpegaudio;

const DECODERS: &[DecoderInfo] = &[
#[cfg(feature="decoder_aac")]
    DecoderInfo { name: "aac", get_decoder: aac::get_decoder },
#[cfg(feature="decoder_mpa")]
    DecoderInfo { name: "mp3", get_decoder: mpegaudio::get_decoder_mp3 },
];

/// Registers all available codecs provided by this crate.
pub fn mpeg_register_all_decoders(rd: &mut RegisteredDecoders) {
    for decoder in DECODERS.iter() {
        rd.add_decoder(*decoder);
    }
}
