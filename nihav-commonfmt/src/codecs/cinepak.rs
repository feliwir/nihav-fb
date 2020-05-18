use nihav_core::io::byteio::{ByteReader,MemoryReader};
use nihav_core::formats::YUV420_FORMAT;
use nihav_core::codecs::*;
use nihav_codec_support::codecs::HAMShuffler;

struct CinepakDecoder {
    info:   NACodecInfoRef,
    frmmgr: HAMShuffler,
    cb_v1:  [[u8; 6]; 256],
    cb_v4:  [[u8; 6]; 256],
}

fn put_block(block: &[u8; 24], x: usize, y: usize, frm: &mut NASimpleVideoFrame<u8>) {
    let mut yoff = frm.offset[0] + x + y * frm.stride[0];
    for i in 0..4 {
        for j in 0..4 {
            frm.data[yoff + j] = block[j + i * 4];
        }
        yoff += frm.stride[0];
    }
    let mut uoff = frm.offset[1] + x / 2 + y / 2 * frm.stride[1];
    for i in 0..2 {
        for j in 0..2 {
            frm.data[uoff + j] = block[j + i * 2 + 16];
        }
        uoff += frm.stride[1];
    }
    let mut voff = frm.offset[2] + x / 2 + y / 2 * frm.stride[2];
    for i in 0..2 {
        for j in 0..2 {
            frm.data[voff + j] = block[j + i * 2 + 20];
        }
        voff += frm.stride[2];
    }
}

impl CinepakDecoder {
    fn new() -> Self {
        CinepakDecoder {
            info:   NACodecInfo::new_dummy(),
            frmmgr: HAMShuffler::new(),
            cb_v1:  [[0; 6]; 256],
            cb_v4:  [[0; 6]; 256],
        }
    }
    fn read_cb(br: &mut ByteReader, size: usize, cb: &mut [[u8; 6]; 256], is_yuv: bool) -> DecoderResult<()> {
        let cb_elem = if is_yuv { 6 } else { 4 };
        let cb_size = (size - 4) / cb_elem;
        validate!(size - 4 == cb_size * cb_elem);
        validate!(cb_size <= 256);
        for i in 0..cb_size {
                                          br.read_buf(&mut cb[i][..cb_elem])?;
            if !is_yuv {
                cb[i][4] = 0x80;
                cb[i][5] = 0x80;
            } else {
                cb[i][4] ^= 0x80;
                cb[i][5] ^= 0x80;
            }
        }
        Ok(())
    }
    fn read_cb_upd(br: &mut ByteReader, size: usize, cb: &mut [[u8; 6]; 256], is_yuv: bool) -> DecoderResult<()> {
        let cb_elem = if is_yuv { 6 } else { 4 };
        let end = br.tell() + (size as u64) - 4;
        for i in (0..256).step_by(32) {
            if br.tell() >= end {
                break;
            }
            let upd                     = br.read_u32be()?;
            for j in 0..32 {
                if ((upd >> (31 - j)) & 1) != 0 {
                                          br.read_buf(&mut cb[i + j][..cb_elem])?;
                    if !is_yuv {
                        cb[i + j][4] = 0x80;
                        cb[i + j][5] = 0x80;
                    } else {
                        cb[i + j][4] ^= 0x80;
                        cb[i + j][5] ^= 0x80;
                    }
                }
            }
        }
        validate!(br.tell() == end);
        Ok(())
    }
    fn decode_strip(&mut self, src: &[u8], is_intra: bool, is_intra_strip: bool, xoff: usize, yoff: usize, xend: usize, yend: usize, frm: &mut NASimpleVideoFrame<u8>) -> DecoderResult<()> {
        let mut mr = MemoryReader::new_read(src);
        let mut br = ByteReader::new(&mut mr);
        let mut idx_pos = 0;
        let mut idx_size = 0;
        let mut v1_only = false;
        while br.left() > 0 {
            let id                      = br.read_byte()?;
            if (id & 0xF0) == 0x20 {
                validate!(((id & 1) != 0) ^ is_intra_strip);
            }
            let size                    = br.read_u24be()? as usize;
            validate!(size >= 4 && (size - 4 <= (br.left() as usize)));
            match id {
                0x20 => Self::read_cb    (&mut br, size, &mut self.cb_v4, true)?,
                0x21 => Self::read_cb_upd(&mut br, size, &mut self.cb_v4, true)?,
                0x22 => Self::read_cb    (&mut br, size, &mut self.cb_v1, true)?,
                0x23 => Self::read_cb_upd(&mut br, size, &mut self.cb_v1, true)?,
                0x24 => Self::read_cb    (&mut br, size, &mut self.cb_v4, false)?,
                0x25 => Self::read_cb_upd(&mut br, size, &mut self.cb_v4, false)?,
                0x26 => Self::read_cb    (&mut br, size, &mut self.cb_v1, false)?,
                0x27 => Self::read_cb_upd(&mut br, size, &mut self.cb_v1, false)?,
                0x30 => { // intra indices
                    validate!(idx_pos == 0);
                    idx_pos = br.tell() as usize;
                    idx_size = size - 4;
                                          br.read_skip(idx_size)?;
                },
                0x31 => { // inter indices
                    validate!(!is_intra);
                    validate!(idx_pos == 0);
                    idx_pos = br.tell() as usize;
                    idx_size = size - 4;
                                          br.read_skip(idx_size)?;
                },
                0x32 => { // V1-only blocks
                    validate!(idx_pos == 0);
                    idx_pos = br.tell() as usize;
                    idx_size = size - 4;
                    v1_only = true;
                                          br.read_skip(idx_size)?;
                },
                _ => return Err(DecoderError::InvalidData),
            };
        }
        validate!(idx_pos != 0);
        let mut mr = MemoryReader::new_read(&src[idx_pos..][..idx_size]);
        let mut br = ByteReader::new(&mut mr);

        let mut x = xoff;
        let mut y = yoff;
        let mut block = [0u8; 24];
        while br.left() > 0 {
            let flags = if !v1_only { br.read_u32be()? } else { 0xFFFFFFFF };
            let mut mask = 1 << 31;
            while mask > 0 {
                if !is_intra {
                    let skip = (flags & mask) == 0;
                    mask >>= 1;
                    if skip {
                        x += 4;
                        if x >= xend {
                            x = xoff;
                            y += 4;
                            if y == yend {
                                return Ok(());
                            }
                        }
                    }
                    continue;
                }
                if (flags & mask) == 0 {
                    let idx         = br.read_byte()? as usize;
                    let cb = &self.cb_v1[idx];
                    block[ 0] = cb[0]; block[ 1] = cb[0]; block[ 2] = cb[1]; block[ 3] = cb[1];
                    block[ 4] = cb[0]; block[ 5] = cb[0]; block[ 6] = cb[1]; block[ 7] = cb[1];
                    block[ 8] = cb[2]; block[ 9] = cb[2]; block[10] = cb[3]; block[11] = cb[3];
                    block[12] = cb[2]; block[13] = cb[2]; block[14] = cb[3]; block[15] = cb[3];
                    block[16] = cb[4]; block[17] = cb[4];
                    block[18] = cb[4]; block[19] = cb[4];
                    block[20] = cb[5]; block[21] = cb[5];
                    block[22] = cb[5]; block[23] = cb[5];
                } else {
                    let idx0        = br.read_byte()? as usize;
                    let cb0 = &self.cb_v4[idx0];
                    let idx1        = br.read_byte()? as usize;
                    let cb1 = &self.cb_v4[idx1];
                    let idx2        = br.read_byte()? as usize;
                    let cb2 = &self.cb_v4[idx2];
                    let idx3        = br.read_byte()? as usize;
                    let cb3 = &self.cb_v4[idx3];
                    block[ 0] = cb0[0]; block[ 1] = cb0[1]; block[ 2] = cb1[0]; block[ 3] = cb1[1];
                    block[ 4] = cb0[2]; block[ 5] = cb0[3]; block[ 6] = cb1[2]; block[ 7] = cb1[3];
                    block[ 8] = cb2[0]; block[ 9] = cb2[1]; block[10] = cb3[0]; block[11] = cb3[1];
                    block[12] = cb2[2]; block[13] = cb2[3]; block[14] = cb3[2]; block[15] = cb3[3];
                    block[16] = cb0[4]; block[17] = cb1[4];
                    block[18] = cb2[4]; block[19] = cb3[4];
                    block[20] = cb0[5]; block[21] = cb1[5];
                    block[22] = cb2[5]; block[23] = cb3[5];
                }
                mask >>= 1;
                put_block(&block, x, y, frm);
                x += 4;
                if x >= xend {
                    x = xoff;
                    y += 4;
                    if y == yend {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}

impl NADecoder for CinepakDecoder {
    fn init(&mut self, _supp: &mut NADecoderSupport, info: NACodecInfoRef) -> DecoderResult<()> {
        if let NACodecTypeInfo::Video(vinfo) = info.get_properties() {
            let w = vinfo.get_width();
            let h = vinfo.get_height();
            let myinfo = NACodecTypeInfo::Video(NAVideoInfo::new(w, h, false, YUV420_FORMAT));
            self.info = NACodecInfo::new_ref(info.get_name(), myinfo, None).into_ref();
            self.frmmgr.clear();
            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, _supp: &mut NADecoderSupport, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let src = pkt.get_buffer();
        if src.len() <= 10 { return Err(DecoderError::ShortData); }

        let mut mr = MemoryReader::new_read(src.as_slice());
        let mut br = ByteReader::new(&mut mr);

        let flags                       = br.read_byte()?;
        let size                        = br.read_u24be()? as usize;
        validate!(src.len() >= size);
        let width                       = br.read_u16be()? as usize;
        let height                      = br.read_u16be()? as usize;
        let nstrips                     = br.read_u16be()? as usize;

        let is_intra = (flags & 1) == 0;

        if let Some(ref vinfo) = self.info.get_properties().get_video_info() {
            if vinfo.width != width || vinfo.height != height {
                let myinfo = NACodecTypeInfo::Video(NAVideoInfo::new(width, height, false, YUV420_FORMAT));
                self.info = NACodecInfo::new_ref(self.info.get_name(), myinfo, None).into_ref();
                self.frmmgr.clear();
            }
        }
        let mut buf;
        if is_intra {
            let vinfo = self.info.get_properties().get_video_info().unwrap();
            let bufinfo = alloc_video_buffer(vinfo, 2)?;
            buf = bufinfo.get_vbuf().unwrap();
        } else {
            let bufret = self.frmmgr.clone_ref();
            if let Some(vbuf) = bufret {
                buf = vbuf;
            } else {
                return Err(DecoderError::MissingReference);
            }
        }
        let mut frm = NASimpleVideoFrame::from_video_buf(&mut buf).unwrap();

        let mut last_y = 0;
        for i in 0..nstrips {
            let flags                   = br.read_byte()?;
            validate!(flags == 0x10 || flags == 0x11);
            let is_intra_strip = (flags & 1) == 0;
            let size                    = br.read_u24be()? as usize;
            validate!(size > 12 && (size - 4) <= (br.left() as usize));
            let yoff                    = br.read_u16be()? as usize;
            let xoff                    = br.read_u16be()? as usize;
            if xoff != 0 || yoff != 0 {
                return Err(DecoderError::NotImplemented);
            }
            let yend                    = br.read_u16be()? as usize;
            let xend                    = br.read_u16be()? as usize;
            if i == 0 && is_intra && !is_intra_strip {
                return Err(DecoderError::InvalidData);
            }
            let start = br.tell() as usize;
            let end = start + size - 12;
            let strip_data = &src[start..end];
            self.decode_strip(strip_data, is_intra, is_intra_strip, 0, last_y, xend, last_y + yend, &mut frm)?;
                                          br.read_skip(size - 12)?;
            last_y += yend;
        }

        self.frmmgr.add_frame(buf.clone());
        let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), NABufferType::Video(buf));
        frm.set_keyframe(is_intra);
        frm.set_frame_type(if is_intra { FrameType::I } else { FrameType::P });
        Ok(frm.into_ref())
    }
    fn flush(&mut self) {
        self.frmmgr.clear();
    }
}

pub fn get_decoder() -> Box<dyn NADecoder + Send> {
    Box::new(CinepakDecoder::new())
}

#[cfg(test)]
mod test {
    use nihav_core::codecs::RegisteredDecoders;
    use nihav_core::demuxers::RegisteredDemuxers;
    use nihav_codec_support::test::dec_video::*;
    use crate::generic_register_all_codecs;
    use crate::generic_register_all_demuxers;
    #[test]
    fn test_cinepak() {
        let mut dmx_reg = RegisteredDemuxers::new();
        generic_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        generic_register_all_codecs(&mut dec_reg);
        test_decoding("avi", "cinepak", "assets/Misc/ot171.avi", Some(10), &dmx_reg,
                     &dec_reg, ExpectedTestResult::MD5Frames(vec![
                        [0xd58326b0, 0xdbfc1dcc, 0x6d66a04c, 0x08a21bbb],
                        [0x9b2cb5c5, 0x69b5f261, 0xcaccaaaf, 0xff2a807d],
                        [0x55c322d5, 0xf76f81ce, 0x923ada8c, 0x4925a5c8],
                        [0x2d1a537a, 0x62233cb6, 0xc1d39c2f, 0xeec9ccf3],
                        [0xf3cc841d, 0x56603c01, 0x34f521cf, 0x61f8a0c9],
                        [0xd75c0802, 0x9e786186, 0xc7a05cdf, 0x52ddc59d],
                        [0xde19733b, 0x29633d17, 0x507e9f82, 0x94c09158],
                        [0x1ea11919, 0x133a282c, 0x8cee485c, 0x150cb3f4],
                        [0x55a6d8fb, 0x2ea287c0, 0x36b3083b, 0x954cfc64],
                        [0xfb8be1fb, 0x84ad10aa, 0xa00ee55c, 0x9e191e5b],
                        [0x9c090a08, 0x43071726, 0x26236b5a, 0x79595848]]));
    }
    #[test]
    fn test_cinepak_gray() {
        let mut dmx_reg = RegisteredDemuxers::new();
        generic_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        generic_register_all_codecs(&mut dec_reg);
        test_decoding("mov", "cinepak", "assets/Misc/dday.mov", Some(10), &dmx_reg,
                     &dec_reg, ExpectedTestResult::MD5Frames(vec![
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x75d4d701, 0x897b4a37, 0xdc2bfb95, 0x3c8871a5],
                        [0x4c67ee48, 0xbea36f9c, 0xde61338b, 0xec36cc90]]));
    }
}
