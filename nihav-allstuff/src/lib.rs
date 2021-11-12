//! Umbrella crate to register decoders and demuxers from all known NihAV crates.
extern crate nihav_core;
extern crate nihav_commonfmt;
extern crate nihav_duck;
extern crate nihav_game;
extern crate nihav_indeo;
extern crate nihav_itu;
extern crate nihav_llaudio;
extern crate nihav_ms;
extern crate nihav_qt;
extern crate nihav_rad;
extern crate nihav_realmedia;
extern crate nihav_vivo;

use nihav_core::codecs::RegisteredDecoders;
use nihav_core::codecs::RegisteredPacketisers;
use nihav_core::codecs::RegisteredEncoders;
use nihav_core::demuxers::RegisteredDemuxers;
use nihav_core::muxers::RegisteredMuxers;

use nihav_commonfmt::*;
use nihav_duck::*;
use nihav_flash::*;
use nihav_game::*;
use nihav_indeo::indeo_register_all_decoders;
use nihav_itu::itu_register_all_decoders;
use nihav_llaudio::*;
use nihav_mpeg::*;
use nihav_ms::*;
use nihav_qt::qt_register_all_decoders;
use nihav_rad::*;
use nihav_realmedia::*;
use nihav_vivo::*;

/// Registers all known decoders.
pub fn nihav_register_all_decoders(rd: &mut RegisteredDecoders) {
    generic_register_all_decoders(rd);
    duck_register_all_decoders(rd);
    flash_register_all_decoders(rd);
    game_register_all_decoders(rd);
    indeo_register_all_decoders(rd);
    itu_register_all_decoders(rd);
    llaudio_register_all_decoders(rd);
    mpeg_register_all_decoders(rd);
    ms_register_all_decoders(rd);
    qt_register_all_decoders(rd);
    rad_register_all_decoders(rd);
    realmedia_register_all_decoders(rd);
    vivo_register_all_decoders(rd);
}

/// Registers all known packetisers.
pub fn nihav_register_all_packetisers(rp: &mut RegisteredPacketisers) {
    mpeg_register_all_packetisers(rp);
}

/// Registers all known demuxers.
pub fn nihav_register_all_demuxers(rd: &mut RegisteredDemuxers) {
    duck_register_all_demuxers(rd);
    generic_register_all_demuxers(rd);
    flash_register_all_demuxers(rd);
    game_register_all_demuxers(rd);
    llaudio_register_all_demuxers(rd);
    rad_register_all_demuxers(rd);
    realmedia_register_all_demuxers(rd);
    vivo_register_all_demuxers(rd);
}

/// Registers all known encoders.
pub fn nihav_register_all_encoders(re: &mut RegisteredEncoders) {
    flash_register_all_encoders(re);
    generic_register_all_encoders(re);
    duck_register_all_encoders(re);
    llaudio_register_all_encoders(re);
    ms_register_all_encoders(re);
}

/// Registers all known demuxers.
pub fn nihav_register_all_muxers(rm: &mut RegisteredMuxers) {
    flash_register_all_muxers(rm);
    generic_register_all_muxers(rm);
    llaudio_register_all_muxers(rm);
}

#[cfg(test)]
extern crate nihav_registry;

#[cfg(test)]
mod test {
    use super::*;
    use nihav_registry::register::get_codec_description;

    #[test]
    fn test_descriptions() {
        let mut rd = RegisteredDecoders::new();
        nihav_register_all_decoders(&mut rd);
        let mut has_missing = false;
        for dec in rd.iter() {
            print!("decoder {} - ", dec.name);
            let ret = get_codec_description(dec.name);
            if let Some(desc) = ret {
                println!("{}", desc);
            } else {
                println!("missing!");
                has_missing = true;
            }
        }
        assert!(!has_missing);
    }
}
