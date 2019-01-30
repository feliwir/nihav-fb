use nihav_core::codecs::*;
use nihav_core::dsp::dct::*;
use nihav_core::dsp::fft::*;
use nihav_core::io::bitreader::*;
use std::f32::consts;
use std::str::FromStr;

enum Transform {
    None,
    DCT(DCT),
    RDFT(RDFT),
}

const MAX_BANDS: usize = 25;

struct BinkAudioDecoder {
    ainfo:      NAAudioInfo,
    chmap:      NAChannelMap,
    use_dct:    bool,
    version_b:  bool,
    transform:  Transform,
    len:        usize,
    quants:     [f32; 96],
    bands:      [usize; MAX_BANDS + 1],
    num_bands:  usize,
    duration:   usize,
    coeffs:     [f32; 4096],
    delay:      [[f32; 256]; 2],
    first_frm:  bool,
    scale:      f32,
}

fn read_bink_float(br: &mut BitReader) -> DecoderResult<f32> {
    /*let exp                                     = (br.read(5)? as i8) - 23;
    let mant                                    = br.read(23)? as u32;
    let sign                                    = br.read_bool()?;
    let v = if exp >= 0 {
            (mant as f32) * ((1 << exp) as f32)
        } else {
            (mant as f32) / ((1 << -exp) as f32)
        };
    if sign {
        Ok(-v)
    } else {
        Ok(v)
    }*/
    let exp                                     = br.read(5)? as u8;
    let mant                                    = br.read(23)? as u32;
    let sign                                    = br.read(1)? as u32;
    let nexp = exp.wrapping_add(0x7E) as u32;
    let nmant = (mant << 1) & ((1 << 23) - 1);
    Ok(f32::from_bits((sign << 31) | (nexp << 23) | nmant))
}

fn overlap(a: &[f32], b: &[f32], dst: &mut [f32], len: usize, step: usize) {
    for i in 0..len {
        dst[i] = (a[i] * ((len - i) as f32) + b[i * step] * (i as f32)) / (len as f32);
    }
}

impl BinkAudioDecoder {
    fn new(use_dct: bool) -> Self {
        let mut quants: [f32; 96] = [0.0; 96];
        for i in 0..quants.len() {
            quants[i] = ((i as f32) * 0.0664 / consts::LOG10_E).exp();
        }
        Self {
            ainfo:      NAAudioInfo::new(0, 1, SND_F32P_FORMAT, 0),
            chmap:      NAChannelMap::new(),
            use_dct,
            transform:  Transform::None,
            version_b:  false,
            len:        0,
            quants:     quants,
            bands:      [MAX_BANDS + 1; 26],
            num_bands:  0,
            duration:   0,
            coeffs:     [0.0; 4096],
            delay:      [[0.0; 256]; 2],
            first_frm:  true,
            scale:      1.0,
        }
    }
    fn decode_block(&mut self, br: &mut BitReader) -> DecoderResult<()> {
        if self.version_b {
            let flt                             = br.read(32)?;
            self.coeffs[0] = f32::from_bits(flt) * self.scale;
            let flt                             = br.read(32)?;
            self.coeffs[1] = f32::from_bits(flt) * self.scale;
        } else {
            self.coeffs[0] = read_bink_float(br)? * self.scale;
            self.coeffs[1] = read_bink_float(br)? * self.scale;
        }
        let mut quants: [f32; MAX_BANDS] = [0.0; MAX_BANDS];
        for i in 0..self.num_bands {
            let idx                             = br.read(8)? as usize;
            quants[i] = self.quants[idx.min(self.quants.len() - 1)] * self.scale;
        }
        let mut idx = 2;
        let mut band_idx = 0;
        self.coeffs = [0.0; 4096];
        while idx < self.len {
            let width = if self.version_b {
                    16
                } else {
                    if br.read_bool()? {
                        let idx                 = br.read(4)? as usize;
                        RUN_TAB[idx] * 8
                    } else {
                        8
                    }
                };
            let end = (idx + width).min(self.len);
            let bits                            = br.read(4)? as u8;
            if bits != 0 {
                for i in idx..end {
                    while self.bands[band_idx] <= i { band_idx += 1; }
                    let q = quants[band_idx - 1];
                    let coeff                   = br.read(bits)?;
                    if coeff != 0 {
                        if br.read_bool()? {
                            self.coeffs[i] = -(coeff as f32) * q;
                        } else {
                            self.coeffs[i] =  (coeff as f32) * q;
                        }
                    }
                }
            }
            idx = end;
        }
        Ok(())
    }
    fn output(&mut self, dst: &mut [f32], off0: usize, off1: usize, chno: usize) {
        match self.transform {
            Transform::DCT(ref mut dct) => {
                    dct.do_dct_inplace(&mut self.coeffs[0..]);
                },
            Transform::RDFT(ref mut rdft) => {
                    unsafe {
                        let buf = &mut self.coeffs[0..];
                        rdft.do_rdft_inplace(std::mem::transmute::<&mut [f32], &mut [FFTComplex]>(buf));
                    }
                },
            _ => unreachable!(),
        };
        if self.use_dct || self.chmap.num_channels() == 1 {
            let overlap_len = if self.first_frm { 0 } else { self.len >> 4 };
            let out = if chno == 0 { &mut dst[off0..] } else { &mut dst[off1..] };
            overlap(&self.delay[chno], &self.coeffs, out, overlap_len, 1);
            let out2 = &mut out[overlap_len..self.duration];
            out2.copy_from_slice(&self.coeffs[overlap_len..self.duration]);
            for i in 0..(self.len >> 4) {
                self.delay[chno][i] = self.coeffs[self.duration + i];
            }
        } else { // somehow it ends here in reverse order
            for i in 0..self.len >> 2 {
                let t0 = self.coeffs[self.len - 2 - i * 2];
                let t1 = self.coeffs[self.len - 1 - i * 2];
                self.coeffs[self.len - 2 - i * 2] = self.coeffs[i * 2];
                self.coeffs[self.len - 1 - i * 2] = self.coeffs[i * 2 + 1];
                self.coeffs[i * 2] = t0;
                self.coeffs[i * 2 + 1] = t1;
            }
            let overlap_len = if self.first_frm { 0 } else { self.len >> 8 };
            overlap(&self.delay[0], &self.coeffs[0..], &mut dst[off0..], overlap_len, 2);
            overlap(&self.delay[1], &self.coeffs[1..], &mut dst[off1..], overlap_len, 2);
            for i in overlap_len..self.duration {
                dst[i + off0] = self.coeffs[i * 2 + 0];
                dst[i + off1] = self.coeffs[i * 2 + 1];
            }
            for i in 0..(self.len >> 8) {
                self.delay[0][i] = self.coeffs[self.duration * 2 + i * 2 + 0];
                self.delay[1][i] = self.coeffs[self.duration * 2 + i * 2 + 1];
            }
        }
    }
}

const CRITICAL_FREQS: [usize; MAX_BANDS] = [
      100,   200,  300,  400,  510,  630,   770,   920,
     1080,  1270, 1480, 1720, 2000, 2320,  2700,  3150,
     3700,  4400, 5300, 6400, 7700, 9500, 12000, 15500,
    24500
];

const RUN_TAB: [usize; 16] = [ 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 32, 64 ];

impl NADecoder for BinkAudioDecoder {
    fn init(&mut self, info: Rc<NACodecInfo>) -> DecoderResult<()> {
        if let NACodecTypeInfo::Audio(ainfo) = info.get_properties() {
            let srate    = ainfo.get_sample_rate();
            let channels = ainfo.get_channels();
            validate!(channels <= 2);
            self.ainfo = NAAudioInfo::new(srate, channels, SND_F32P_FORMAT, 2);
            self.chmap = NAChannelMap::from_str("L,R").unwrap();
            if let Some(ref edata) = info.get_extradata() {
                if edata.as_slice() == b"BIKb" {
                    self.version_b = true;
                }
            } else {
                self.version_b = false;
            }
            let mut frame_bits = if srate < 22050 { 9 } else if srate < 44100 { 10 } else { 11 };
            if !self.use_dct && !self.version_b {
                frame_bits += 1;
            }
            self.len = 1 << frame_bits;
            self.duration = self.len - (self.len >> 4);
            let single = !self.use_dct && channels == 2; // RDFT codes samples interleaved as single buffer
            if single {
                self.duration >>= 1;
            }
            self.transform = if !self.use_dct {
                    Transform::RDFT(RDFTBuilder::new_rdft(FFTMode::SplitRadix, self.len >> 1, false))
                } else {
                    Transform::DCT(DCT::new(DCTMode::DCT_III, self.len))
                };
            self.scale = if !self.use_dct {
                    1.0 / (32768.0 * (self.len as f32).sqrt())
                } else {
                    (2.0 / (self.len as f32)).sqrt() / 1024.0
                };
            let s_srate = if single { srate } else { srate >> 1 } as usize;
            self.num_bands = 1;
            while self.num_bands < CRITICAL_FREQS.len() && CRITICAL_FREQS[self.num_bands - 1] < s_srate {
                self.num_bands += 1;
            }
            self.bands[0] = 2;
            for i in 1..self.num_bands {
                self.bands[i] = (CRITICAL_FREQS[i - 1] * self.len / s_srate) & !1;
            }
            self.bands[self.num_bands] = self.len;
            self.first_frm = true;

            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let info = pkt.get_stream().get_info();
        if let NACodecTypeInfo::Audio(_) = info.get_properties() {
            let pktbuf = pkt.get_buffer();
            validate!(pktbuf.len() > 1);
            let mut br = BitReader::new(&pktbuf, pktbuf.len(), BitReaderMode::LE);
            let nsamples = br.read(32)? as usize;
//            validate!(nsamples % self.duration == 0);

            let mut abuf = alloc_audio_buffer(self.ainfo, nsamples / self.chmap.num_channels() / 2, self.chmap.clone())?;
            let mut adata = abuf.get_abuf_f32().unwrap();
            let mut off0 = adata.get_offset(0);
            let mut off1 = adata.get_offset(1);
            let mut dst = adata.get_data_mut();

            let num_subframes = nsamples / self.duration / self.chmap.num_channels() / 2;

            for _subfrm in 0..num_subframes {
                if self.use_dct {
                                                    br.skip(2)?;
                }
                self.decode_block(&mut br)?;
                self.output(&mut dst[0..], off0, off1, 0);
                if self.chmap.num_channels() > 1 && self.use_dct {
                    self.decode_block(&mut br)?;
                    self.output(&mut dst[0..], off0, off1, 1);
                }
                self.first_frm = false;
                let left = br.left() & 31;
                if left != 0 {
                    br.skip(left as u32)?;
                }
                off0 += self.duration;
                off1 += self.duration;
            }

            let mut frm = NAFrame::new_from_pkt(pkt, info, abuf);
            frm.set_duration(Some(self.duration as u64));
            frm.set_keyframe(false);
            Ok(Rc::new(RefCell::new(frm)))
        } else {
            Err(DecoderError::InvalidData)
        }
    }
}

pub fn get_decoder_dct() -> Box<NADecoder> {
    Box::new(BinkAudioDecoder::new(true))
}

pub fn get_decoder_rdft() -> Box<NADecoder> {
    Box::new(BinkAudioDecoder::new(false))
}

#[cfg(test)]
mod test {
    use nihav_core::codecs::RegisteredDecoders;
    use nihav_core::demuxers::RegisteredDemuxers;
    use nihav_core::test::dec_video::*;
    use crate::codecs::rad_register_all_codecs;
    use crate::demuxers::rad_register_all_demuxers;

    #[test]
    fn test_bink_audio() {
        let mut dmx_reg = RegisteredDemuxers::new();
        rad_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        rad_register_all_codecs(&mut dec_reg);

//        let file = "assets/RAD/ActivisionLogo.bik";
        let file = "assets/RAD/original.bik";
//        let file = "assets/RAD/Snd0a110c51.dee";
//        let file = "assets/RAD/NEW.BIK";
//        let file = "assets/RAD/ge_video_86l.bk2";
        test_decode_audio("bink", file, None, "bink", &dmx_reg, &dec_reg);
    }
}