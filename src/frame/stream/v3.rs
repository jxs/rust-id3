extern crate flate;
extern crate audiotag;

use frame::stream::FrameStream;
use frame::Frame;
use audiotag::{TagResult, TagError, UnsupportedFeatureError, StringDecodingError};
use util;

pub struct FrameV3;
impl FrameStream for FrameV3 {
    fn read(reader: &mut Reader, _: Option<FrameV3>) -> TagResult<Option<(u32, Frame)>> {
        let mut frame = Frame::with_version("", 3);

        frame.id = id_or_padding!(reader, 4);
        debug!("reading {}", frame.id); 

        let content_size = try!(reader.read_be_u32());

        let frameflags = try!(reader.read_be_u16());
        frame.flags.tag_alter_preservation = frameflags & 0x8000 != 0;
        frame.flags.file_alter_preservation = frameflags & 0x4000 != 0;
        frame.flags.read_only = frameflags & 0x2000 != 0;
        frame.flags.compression = frameflags & 0x80 != 0;
        frame.flags.encryption = frameflags & 0x40 != 0;
        frame.flags.grouping_identity = frameflags & 0x20 != 0;

        if frame.flags.encryption {
            debug!("[{}] encryption is not supported", frame.id);
            return Err(TagError::new(UnsupportedFeatureError, "encryption is not supported"));
        } else if frame.flags.grouping_identity {
            debug!("[{}] grouping identity is not supported", frame.id);
            return Err(TagError::new(UnsupportedFeatureError, "grouping identity is not supported"));
        }

        let mut read_size = content_size;
        if frame.flags.compression {
            let _decompressed_size = try!(reader.read_be_u32());
            read_size -= 4;
        }
        
        let data = try!(reader.read_exact(read_size as uint));
        try!(frame.parse_data(data.as_slice()));

        Ok(Some((10 + content_size, frame)))
    }

    fn write(writer: &mut Writer, frame: &Frame, _: Option<FrameV3>) -> TagResult<u32> {
        let mut content_bytes = frame.contents_to_bytes();
        let mut content_size = content_bytes.len() as u32;
        let decompressed_size = content_size;

        if frame.flags.compression {
            debug!("[{}] compressing frame contents", frame.id);
            content_bytes = flate::deflate_bytes_zlib(content_bytes.as_slice()).unwrap().as_slice().to_vec();
            content_size = content_bytes.len() as u32 + 4;
        }

        try!(writer.write(frame.id.slice_to(4).as_bytes()));
        try!(writer.write(util::u32_to_bytes(content_size).as_slice()));
        try!(writer.write(frame.flags.to_bytes(0x3).as_slice()))
        if frame.flags.compression {
            try!(writer.write(util::u32_to_bytes(decompressed_size).as_slice()));
        }
        try!(writer.write(content_bytes.as_slice()));

        Ok(10 + content_size)
    }
}

