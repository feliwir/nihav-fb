use nihav_core::codecs::*;

macro_rules! validate {
    ($a:expr) => { if !$a { println!("check failed at {}:{}", file!(), line!()); return Err(DecoderError::InvalidData); } };
}

#[cfg(feature="decoder_truemotion1")]
mod truemotion1;
#[cfg(feature="decoder_truemotionrt")]
mod truemotionrt;
#[cfg(feature="decoder_truemotion2")]
#[allow(clippy::needless_range_loop)]
mod truemotion2;
#[cfg(feature="decoder_truemotion2x")]
mod truemotion2x;
#[cfg(any(feature="decoder_vp3", feature="decoder_vp4", feature="decoder_vp5", feature="decoder_vp6", feature="decoder_vp7"))]
#[macro_use]
#[allow(clippy::erasing_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::useless_let_if_seq)]
mod vpcommon;
#[cfg(any(feature="decoder_vp3", feature="decoder_vp4"))]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
mod vp3;
#[cfg(any(feature="decoder_vp5", feature="decoder_vp6"))]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::useless_let_if_seq)]
#[allow(clippy::too_many_arguments)]
mod vp56;
#[cfg(feature="decoder_vp5")]
#[allow(clippy::needless_range_loop)]
mod vp5;
#[cfg(any(feature="decoder_vp6", feature="encoder_vp6"))]
mod vp6data;
#[cfg(any(feature="decoder_vp6", feature="encoder_vp6"))]
mod vp6dsp;
#[cfg(feature="decoder_vp6")]
#[allow(clippy::needless_range_loop)]
mod vp6;
#[cfg(feature="decoder_vp7")]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::useless_let_if_seq)]
mod vp7;
#[cfg(any(feature="decoder_vp7", feature="decoder_vp8"))]
mod vp78data;
#[cfg(feature="decoder_vp7")]
#[allow(clippy::erasing_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::useless_let_if_seq)]
mod vp7dsp;
#[cfg(any(feature="decoder_vp7", feature="decoder_vp8"))]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::useless_let_if_seq)]
mod vp78;
#[cfg(any(feature="decoder_vp7", feature="decoder_vp8"))]
#[allow(clippy::erasing_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::useless_let_if_seq)]
mod vp78dsp;
#[cfg(feature="decoder_vp8")]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::useless_let_if_seq)]
mod vp8;
#[cfg(feature="decoder_vp8")]
#[allow(clippy::erasing_op)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
mod vp8dsp;

#[cfg(any(feature="decoder_dk3_adpcm", feature="decoder_dk4_adpcm"))]
mod dkadpcm;
#[cfg(feature="decoder_on2avc")]
#[allow(clippy::manual_memcpy)]
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
mod on2avc;
#[cfg(feature="decoder_on2avc")]
mod on2avcdata;

const DUCK_CODECS: &[DecoderInfo] = &[
#[cfg(feature="decoder_truemotion1")]
    DecoderInfo { name: "truemotion1", get_decoder: truemotion1::get_decoder },
#[cfg(feature="decoder_truemotionrt")]
    DecoderInfo { name: "truemotionrt", get_decoder: truemotionrt::get_decoder },
#[cfg(feature="decoder_truemotion2")]
    DecoderInfo { name: "truemotion2", get_decoder: truemotion2::get_decoder },
#[cfg(feature="decoder_truemotion2x")]
    DecoderInfo { name: "truemotion2x", get_decoder: truemotion2x::get_decoder },
#[cfg(feature="decoder_vp3")]
    DecoderInfo { name: "vp3", get_decoder: vp3::get_decoder_vp3 },
#[cfg(feature="decoder_vp4")]
    DecoderInfo { name: "vp4", get_decoder: vp3::get_decoder_vp4 },
#[cfg(feature="decoder_vp5")]
    DecoderInfo { name: "vp5", get_decoder: vp5::get_decoder },
#[cfg(feature="decoder_vp6")]
    DecoderInfo { name: "vp6", get_decoder: vp6::get_decoder_vp6 },
#[cfg(feature="decoder_vp6")]
    DecoderInfo { name: "vp6f", get_decoder: vp6::get_decoder_vp6f },
#[cfg(feature="decoder_vp6")]
    DecoderInfo { name: "vp6a", get_decoder: vp6::get_decoder_vp6_alpha },
#[cfg(feature="decoder_vp7")]
    DecoderInfo { name: "vp7", get_decoder: vp7::get_decoder },
#[cfg(feature="decoder_vp8")]
    DecoderInfo { name: "vp8", get_decoder: vp8::get_decoder },

#[cfg(feature="decoder_dk3_adpcm")]
    DecoderInfo { name: "adpcm-dk3", get_decoder: dkadpcm::get_decoder_dk3 },
#[cfg(feature="decoder_dk4_adpcm")]
    DecoderInfo { name: "adpcm-dk4", get_decoder: dkadpcm::get_decoder_dk4 },
#[cfg(feature="decoder_on2avc")]
    DecoderInfo { name: "on2avc-500", get_decoder: on2avc::get_decoder_500 },
#[cfg(feature="decoder_on2avc")]
    DecoderInfo { name: "on2avc-501", get_decoder: on2avc::get_decoder_501 },
];

/// Registers all available codecs provided by this crate.
pub fn duck_register_all_decoders(rd: &mut RegisteredDecoders) {
    for decoder in DUCK_CODECS.iter() {
        rd.add_decoder(*decoder);
    }
}

#[cfg(feature="encoder_vp6")]
#[allow(clippy::needless_range_loop)]
mod vp6enc;

const DUCK_ENCODERS: &[EncoderInfo] = &[
#[cfg(feature="encoder_vp6")]
    EncoderInfo { name: "vp6", get_encoder: vp6enc::get_encoder },
#[cfg(feature="encoder_vp6")]
    EncoderInfo { name: "vp6f", get_encoder: vp6enc::get_encoder_flv },
];

/// Registers all available encoders provided by this crate.
pub fn duck_register_all_encoders(re: &mut RegisteredEncoders) {
    for encoder in DUCK_ENCODERS.iter() {
        re.add_encoder(*encoder);
    }
}
