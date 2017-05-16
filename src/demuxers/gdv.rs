use super::*;
use io::byteio::*;
use frame::*;
use formats::*;
//use std::collections::HashMap;

enum GDVState {
    NewFrame,
    AudioRead,
}

#[allow(dead_code)]
pub struct GremlinVideoDemuxer<'a> {
    opened:     bool,
    src:        &'a mut ByteReader<'a>,
    frames:     u16,
    cur_frame:  u16,
    asize:      usize,
    apacked:    bool,
    state:      GDVState,
    pktdta:     Vec<u8>,
    dmx:        Demuxer,
    a_id:       Option<usize>,
    v_id:       Option<usize>,
}

impl<'a> Demux<'a> for GremlinVideoDemuxer<'a> {
    #[allow(unused_variables)]
    fn open(&mut self) -> DemuxerResult<()> {
        let src = &mut self.src;
        let magic = src.read_u32le()?;
        if magic != 0x29111994 { return Err(DemuxerError::InvalidData); }
        let id = src.read_u16le()?;
        let frames = src.read_u16le()?;
        let fps = src.read_u16le()?;
        let aflags = src.read_u16le()?;
        let rate = src.read_u16le()?;
        let depth = src.read_u16le()?;
        let max_fs = src.read_u16le()?;
        src.read_skip(2)?;
        let width = src.read_u16le()?;
        let height = src.read_u16le()?;
        if max_fs > 0 {
            let vhdr = NAVideoInfo::new(width as u32, height as u32, false, PAL8_FORMAT);
            let vci = NACodecTypeInfo::Video(vhdr);
            let vinfo = NACodecInfo::new("video-gdv", vci, None);
            self.v_id = self.dmx.add_stream(NAStream::new(StreamType::Video, 0, vinfo));
        }
        if (aflags & 1) != 0 {
            let channels = if (aflags & 2) != 0 { 2 } else { 1 };
            let ahdr = NAAudioInfo::new(rate as u32, channels as u8, if (aflags & 4) != 0 { SND_S16_FORMAT } else { SND_U8_FORMAT }, 2);
            let ainfo = NACodecInfo::new("audio-gdv", NACodecTypeInfo::Audio(ahdr), None);
            self.a_id = self.dmx.add_stream(NAStream::new(StreamType::Audio, 1, ainfo));

            let packed = if (aflags & 8) != 0 { 1 } else { 0 };
            self.asize = (((rate / fps) * channels * (if (aflags & 4) != 0 { 2 } else { 1 })) >> packed) as usize;
            self.apacked = (aflags & 8) != 0;
        }
        if max_fs > 0 && depth == 1 {
            src.read_skip(768)?;
        }
        self.frames = frames;
        self.opened = true;
        self.state = GDVState::NewFrame;
        Ok(())
    }

    #[allow(unused_variables)]
    fn get_frame(&mut self) -> DemuxerResult<NAPacket> {
        if !self.opened { return Err(DemuxerError::NoSuchInput); }
        if self.cur_frame >= self.frames { return Err(DemuxerError::EOF); }
        match self.state {
            GDVState::NewFrame if self.asize > 0 => { self.read_achunk() }
            _ => { self.read_vchunk() }
        }
    }

    #[allow(unused_variables)]
    fn seek(&mut self, time: u64) -> DemuxerResult<()> {
        if !self.opened { return Err(DemuxerError::NoSuchInput); }
        Err(DemuxerError::NotImplemented)
    }
}
/*impl<'a> Drop for GremlinVideoDemuxer<'a> {
    #[allow(unused_variables)]
    fn drop(&mut self) {
    }
}*/
impl<'a> GremlinVideoDemuxer<'a> {
    pub fn new(io: &'a mut ByteReader<'a>) -> Self {
        GremlinVideoDemuxer {
            cur_frame: 0,
            frames: 0,
            opened: false,
            asize: 0,
            apacked: false,
            state: GDVState::NewFrame,
pktdta: Vec::new(),
            src: io,
            a_id: None,
            v_id: None,
            dmx: Demuxer::new()
        }
    }

    fn read_achunk(&mut self) -> DemuxerResult<NAPacket> {
        self.state = GDVState::AudioRead;
        let str = self.dmx.get_stream(self.a_id.unwrap()).unwrap();
        self.src.read_packet(str, Some(self.cur_frame as u64), None, None, true, self.asize)
    }

    fn read_vchunk(&mut self) -> DemuxerResult<NAPacket> {
        let str = self.dmx.get_stream(self.v_id.unwrap()).unwrap();
        let mut src = &mut self.src;
        let magic = src.read_u16be()?;
        if magic != 0x0513 { return Err(DemuxerError::InvalidData); }
        let size = (src.read_u16le()? as usize) + 4;
        let tmp = src.peek_u32le()?;
        let flags = (tmp & 0xFF) as usize;
        self.state = GDVState::NewFrame;
        self.cur_frame = self.cur_frame + 1;
        src.read_packet(str, Some((self.cur_frame - 1) as u64), None, None, if (flags & 64) != 0 { true } else { false }, size)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_gdv_demux() {
        let mut file = File::open("assets/intro1.gdv").unwrap();
        let mut fr = FileReader::new_read(&mut file);
        let mut br = ByteReader::new(&mut fr);
        let mut dmx = GremlinVideoDemuxer::new(&mut br);
        dmx.open().unwrap();
        loop {
            let pktres = dmx.get_frame();
            if let Err(e) = pktres {
                if (e as i32) == (DemuxerError::EOF as i32) { break; }
                panic!("error");
            }
            let pkt = pktres.unwrap();
            println!("Got {}", pkt);
        }
    }
}
