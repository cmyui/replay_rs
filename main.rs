/* messy replay parser, first rust project :D */

use std::io::Read;

#[allow(dead_code)]
pub struct BinaryReader {
    data: Vec<u8>,
    offs: usize,
}

macro_rules! create_read_method {
    ($func:ident, $ty:ty) => {
        #[inline]
        pub fn $func(&mut self) -> $ty {
            let val: $ty = unsafe {
                *(self.data[self.offs..self.offs+std::mem::size_of::<$ty>()].as_mut_ptr() as *mut $ty)
            };
            self.offs += std::mem::size_of::<$ty>();
            val
        }
    };
}

impl BinaryReader {
    #[inline]
    pub fn read(&mut self, len: usize) -> &[u8] {
        let val = &self.data[self.offs..self.offs+len];
        self.offs += len;
        val
    }

    #[inline]
    pub fn read_u8(&mut self) -> u8 {
        let val: u8 = self.data[self.offs];
        self.offs += 1;
        val
    }

    create_read_method!(read_i16, i16);
    create_read_method!(read_u16, u16);
    create_read_method!(read_i32, i32);
    create_read_method!(read_u32, u32);
    create_read_method!(read_i64, i64);
    create_read_method!(read_u64, u64);
    create_read_method!(read_i128, i128);
    create_read_method!(read_u128, u128);

    create_read_method!(read_f32, f32);
    create_read_method!(read_f64, f64);

    #[inline]
    fn read_uleb128(&mut self) -> usize {
        let mut val: usize = 0;
        let mut shift = 0;
        loop {
            let b = self.read_u8();
            val |= (b as usize & 127) << shift;
            if (b & 128) == 0 {
                break;
            }
            shift += 7
        }
        val
    }

    #[inline]
    pub fn read_str_uleb128(&mut self) -> String {
        if self.read_u8() != 0x0b {
            return String::default();
        }

        let len = self.read_uleb128();

        let val = String::from_utf8_lossy(
            &self.data[self.offs..self.offs+len]
        ).into_owned();
        self.offs += len;
        val
    }
}

#[allow(dead_code)]
pub struct ReplayFrame {
    delta: i32,
    x: f32,
    y: f32,
    keys: i32,
}

#[allow(dead_code)]
pub struct Replay {
    mode: u8,
    osu_version: i32,
    map_md5: String,
    player_name: String,
    replay_md5: String,
    n300: i16,
    n100: i16,
    n50: i16,
    ngeki: i16,
    nkatu: i16,
    nmiss: i16,
    score: i32,
    max_combo: i16,
    perfect: bool,
    mods: i32, // TODO struct
    life_graph: String, // TODO: vec/array of tuples?
    timestamp: i64,
    frames: Vec<ReplayFrame>,
    score_id: i64,
    mod_extras: f64,
    seed: i32,
}

#[allow(dead_code)]
impl Replay {
    pub fn from_data(data: Vec<u8>) -> std::io::Result<Replay> {
        let mut reader = BinaryReader { data: data, offs: 0 };

        // read replay headers
        let mut replay = Replay {
            mode: reader.read_u8(),
            osu_version: reader.read_i32(),
            map_md5: reader.read_str_uleb128(),
            player_name: reader.read_str_uleb128(),
            replay_md5: reader.read_str_uleb128(),
            n300: reader.read_i16(),
            n100: reader.read_i16(),
            n50: reader.read_i16(),
            ngeki: reader.read_i16(),
            nkatu: reader.read_i16(),
            nmiss: reader.read_i16(),
            score: reader.read_i32(),
            max_combo: reader.read_i16(),
            perfect: reader.read_u8() == 1,
            mods: reader.read_i32(),
            life_graph: reader.read_str_uleb128(),
            timestamp: reader.read_i64(),

            // not yet parsed
            frames: Vec::<ReplayFrame>::new(),
            score_id: 0,
            mod_extras: 0.0,
            seed: 0,
        };

        // read lzma-encrypted replay frames
        let lzma_len = reader.read_i32() as usize;
        let lzma_data = reader.read(lzma_len);

        // create a decompressor
        let stream = xz2::stream::Stream::new_lzma_decoder(u64::MAX).unwrap();
        let mut decompressor = xz2::read::XzDecoder::new_stream(lzma_data, stream);

        // alloc space for decompressed frames & read into it
        let mut raw_data = String::with_capacity(lzma_len * 5);//usually around ~5x
        decompressor.read_to_string(&mut raw_data)?;
        raw_data.shrink_to_fit();
        raw_data.pop(); // ,

        // parse frames into struct objects
        for action in raw_data.split(|c| c == ',') {
            let split: Vec<&str> = action.split(|c| c == '|').collect();

            if split[0] != "-12345" {
                // normal replay frame
                replay.frames.push(ReplayFrame{
                    delta: split[0].parse::<i32>().unwrap(),
                    x: split[1].parse::<f32>().unwrap(),
                    y: split[2].parse::<f32>().unwrap(),
                    keys: split[3].parse::<i32>().unwrap(),
                });
            } else {
                // special case - final frame contains seed (used for mania random mod)
                // XXX: this could be optimized/cleaned out of the loop
                replay.seed = split[3].parse::<i32>().unwrap();
            }
        }

        // read replay trailers
        replay.score_id = reader.read_i64(); // is an i32 for <2012/10/08

        if replay.mods & 1 << 23 != 0 { // target
            reader.read_f64();
        }

        println!("raw_len/lzma_len = {}", raw_data.len() as f32 / lzma_len as f32);
        Ok(replay)
    }

    #[inline]
    pub fn from_file(path: &str) -> std::io::Result<Replay> {
        let mut file = std::fs::File::open(path)?;
        let mut buf = Vec::<u8>::with_capacity(file.metadata()?.len() as usize);
        file.read_to_end(&mut buf)?;
        Replay::from_data(buf)
    }
}

fn main() -> () {
    let replay = Replay::from_file("rrr.osr");
}
