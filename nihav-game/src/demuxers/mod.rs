use nihav_core::demuxers::*;

#[allow(unused_macros)]
macro_rules! validate {
    ($a:expr) => { if !$a { println!("check failed at {}:{}", file!(), line!()); return Err(DemuxerError::InvalidData); } };
}

#[cfg(any(feature="demuxer_bmv",feature="demuxer_bmv3"))]
mod bmv;
#[cfg(feature="demuxer_gdv")]
mod gdv;
#[cfg(feature="demuxer_vmd")]
mod vmd;

const GAME_DEMUXERS: &[&'static DemuxerCreator] = &[
#[cfg(feature="demuxer_bmv")]
    &bmv::BMVDemuxerCreator {},
#[cfg(feature="demuxer_bmv3")]
    &bmv::BMV3DemuxerCreator {},
#[cfg(feature="demuxer_gdv")]
    &gdv::GDVDemuxerCreator {},
#[cfg(feature="demuxer_vmd")]
    &vmd::VMDDemuxerCreator {},
];

pub fn game_register_all_demuxers(rd: &mut RegisteredDemuxers) {
    for demuxer in GAME_DEMUXERS.into_iter() {
        rd.add_demuxer(*demuxer);
    }
}
