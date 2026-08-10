use crate::Identification;
use crate::Serialization;
use crate::Deserialization;

use std::cmp::min;
use std::collections::VecDeque;
use std::io::Cursor;
use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use godot::prelude::*;
//use godot::classes::ImageTexture;
//use godot::classes::Image;
//use godot::classes::image::Format;
//use crate::bin_palette::BinPalette;
use crate::sprite_transform;
//use crate::sprite_compress;
//use crate::sprite_compress::{extract_bits, get_palette, pop_bits, CompressedData, SpriteData};
//use crate::sprite_compress::SpriteData;

//pub const HEADER_SIZE: usize = 16;

enum Mode {
	Raw,
	ACPR,
	Palette,
	Mode5,
	GGXP,
}

enum CLUT {
	None,
	Half,
	Full,
}

// Header signature addresses
const ADDRESS_MODE			: usize = 0x00;
const ADDRESS_CLUT			: usize = 0x02;
const ADDRESS_DEPTH			: usize = 0x04;
const ADDRESS_WIDTH			: usize = 0x06;
const ADDRESS_HEIGHT		: usize = 0x08;
const ADDRESS_TEX_WIDTH		: usize = 0x0A;
const ADDRESS_TEX_HEIGHT	: usize = 0x0C;
const ADDRESS_HASH			: usize = 0x0E;

const ADDRESS_GGXP_CC		: usize = 0x01;	// 'C'ompression and 'C'LUT location for GGX Plus sprites
const ADDRESS_GGXP_WIDTH	: usize = 0x02;
const ADDRESS_GGXP_HEIGHT	: usize = 0x04;

const ADDRESS_HEADER_END	: usize = 0x10;

// Mode values
const MODE_RAW			: u16 = 0x0000;
const MODE_ACPR			: u16 = 0x0001;
const MODE_PALETTE		: u16 = 0x0003;
const MODE_5			: u16 = 0x0005;
const MODE_GGXP8		: u8 = 0x13;
const MODE_GGXP4		: u8 = 0x14;

// GGX Plus compression values
const GGXP_UNCOMPRESSED	: u8 = 0x00;
const GGXP_COMPRESSED	: u8 = 0x04;

// CLUT values
const CLUT_NONE			: u16 = 0x0000;
const CLUT_HALF			: u16 = 0x0010;
const CLUT_FULL			: u16 = 0x0020;

// GGX Plus CLUT values
const GGXP_CLUT_NONE	: u8 = 0x0F;
const GGXP_CLUT_HALF	: u8 = 0x02;
const GGXP_CLUT_FULL	: u8 = 0x00;

// Depth values
const DEPTH_4			: u16 = 0x0004;
const DEPTH_8			: u16 = 0x0008;

// Common format
const COMMON_MODES		: [u16; 4] = [MODE_RAW, MODE_ACPR, MODE_PALETTE, MODE_5];
const COMMON_CLUT		: [u16; 3] = [CLUT_NONE, CLUT_HALF, CLUT_FULL];
const COMMON_DEPTH		: [u16; 2] = [DEPTH_4, DEPTH_8];

// GGX Plus sprites
const GGXP_MODES		: [u8; 2] = [MODE_GGXP4, MODE_GGXP8];
const GGXP_COMPRESSION	: [u8; 2] = [GGXP_UNCOMPRESSED, GGXP_COMPRESSED];
const GGXP_CLUT			: [u8; 3] = [GGXP_CLUT_NONE, GGXP_CLUT_HALF, GGXP_CLUT_FULL];

pub struct BinHeader {
	pub compressed: bool,
	pub mode: u16,
	pub clut: u16,
	pub bit_depth: u16,
	pub width: u16,
	pub height: u16,
	pub tw: u16,
	pub th: u16,
	pub hash: u16,
}

/*
pub fn get_header(data: Vec<u8>) -> BinHeader {
	let compressed: bool;
	let mode: u16 = u16::from_le_bytes([data[0x00], data[0x01]]);
	let clut: u16;
	let bit_depth: u16;
	let width: u16;
	let height: u16;

	// GGX
	if data[0x00] > 0x05 {
		// If high nibble of high byte is non-zero
		compressed = (mode & 0xF000) >> 8 != 0;

		// If low nibble of high byte is zero
		match (mode & 0xF00) >> 8 {
			0x0F => clut = 0x00,
			0x00 => clut = 0x20,
			_ => clut = 0x10,
		}

		// If low nibble of low byte is 3
		if (mode & 0xF) == 3 {
			bit_depth = 8
		} else {
			bit_depth = 4
		}

		width = u16::from_le_bytes([
			data[0x02], data[0x03]
		]);

		height = u16::from_le_bytes([
			data[0x04], data[0x05]
		]);
	} else {
		compressed = mode > 0x00;

		clut = u16::from_le_bytes([
			data[0x02], data[0x03]
		]);
		
		bit_depth = u16::from_le_bytes([
			data[0x04], data[0x05]
		]);
		
		width = u16::from_le_bytes([
			data[0x06], data[0x07]
		]);
		
		height = u16::from_le_bytes([
			data[0x08], data[0x09]
		]);
	}

	return BinHeader {
		compressed,
		mode,
		clut,
		bit_depth,
		width,
		height,
		
		tw: u16::from_le_bytes([
			data[0x0A], data[0x0B]
		]),
		
		th: u16::from_le_bytes([
			data[0x0C], data[0x0D]
		]),
		
		hash: u16::from_le_bytes([
			data[0x0E], data[0x0F]
		]),
	}
}


pub fn make_header(compressed: bool, clut: u16, bit_depth: u16, width: u16, height: u16, tw: u16, th: u16, hash: u16) -> Vec<u8> {
	let mut return_vector: Vec<u8> = Vec::with_capacity(0x10);
	
	// mode (compressed/uncompressed)
	return_vector.push(compressed as u8);
	return_vector.push(0x00);
	
	// clut (embedded palette)
	return_vector.extend_from_slice(&clut.to_le_bytes());
	
	// pix (bit depth)
	return_vector.extend_from_slice(&bit_depth.to_le_bytes());
	
	// width
	return_vector.extend_from_slice(&width.to_le_bytes());
	
	// height
	return_vector.extend_from_slice(&height.to_le_bytes());
	
	// tw (unknown)
	return_vector.extend_from_slice(&tw.to_le_bytes());
	
	// th (unknown)
	return_vector.extend_from_slice(&th.to_le_bytes());
	
	// hash (generation method unknown, doesn't affect result)
	return_vector.extend_from_slice(&hash.to_le_bytes());
	
	return return_vector;
}
*/

pub fn generate_hash(bin_data: Vec<u8>) -> u16 {
	let mut hash: u16 = 0;

	for byte in 0..bin_data.len() / 2 {
		hash = hash ^ u16::from_le_bytes([
			bin_data[byte + 0],
			bin_data[byte + 1],
		]);
	}
	
	return hash;
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Data resulting from loading a sprite_#.bin file.
pub struct BinSprite {
	base: Base<Resource>,
	/// The mode the sprite was imported with - Raw, AC+R, Palette, Chunks (Mode 5), Packed (GGX Plus)
	mode: Mode,
	/// Embedded palette mode: none, half-size, or full-size.
	clut: CLUT,
	/// The sprite's bit depth: 4 or 8.
	bit_depth: u16,
	/// The sprite's width.
	width: u16,
	/// The sprite's height.
	height: u16,
	/// The (power of 2) width of the texture that will be allocated.
	texture_width: u16,
	/// The (power of 2) height of the texture that will be allocated.
	texture_height: u16,
	/// The sprite's hash. Not used by all sprites.
	hash: u16,
	/// When true, do not recalculate the sprite's hash when it is updated.
	manual_hash: bool,
	/// The sprite's embedded palette.
	palette: Vec<u8>,
	/// The sprite's pixel vector.
	pixels: Vec<u8>
}


#[godot_api]
impl IResource for BinSprite {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			mode: Mode::ACPR,
			clut: CLUT::None,
			bit_depth: DEPTH_8,
			width: 0,
			height: 0,
			texture_width: 0,
			texture_height: 0,
			hash: 0,
			manual_hash: false,
			palette: Vec::new(),
			pixels: Vec::new(),
		}
	}
}


impl Identification for BinSprite {
	fn identify(bin_data: &Vec<u8>) -> bool {
		if GGXP_MODES.contains(&bin_data[ADDRESS_MODE]) {
			if !GGXP_COMPRESSION.contains(&(bin_data[ADDRESS_MODE + 0x01] >> 0x4))
			{ return false; }

			if !GGXP_CLUT.contains(&(bin_data[ADDRESS_MODE + 0x01] & 0xF))
			{ return false; }

			return true;
		}

		let mode: &u16 = &u16::from_le_bytes([
			bin_data[ADDRESS_MODE + 0],
			bin_data[ADDRESS_MODE + 1],
		]);

		let clut: &u16 = &u16::from_le_bytes([
			bin_data[ADDRESS_CLUT + 0],
			bin_data[ADDRESS_CLUT + 1],
		]);

		let bpp: &u16 = &u16::from_le_bytes([
			bin_data[ADDRESS_DEPTH + 0],
			bin_data[ADDRESS_DEPTH + 1],
		]);

		if !COMMON_MODES.contains(mode)	{ return false; }
		if !COMMON_CLUT.contains(clut)	{ return false; }
		if !COMMON_DEPTH.contains(bpp)	{ return false; }

		return true;
	}
}


impl Serialization for BinSprite {
	// Output is +R-only for now. If other formats get supported, this will change.
	fn serialize(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		// Mode
		match self.mode {
			Mode::Raw		=> bin_data.extend(MODE_RAW.to_le_bytes()),
			Mode::Palette	=> bin_data.extend(MODE_PALETTE.to_le_bytes()),
			_				=> bin_data.extend(MODE_ACPR.to_le_bytes()),
		}

		// CLUT
		match self.clut {
			CLUT::None => bin_data.extend(CLUT_NONE.to_le_bytes()),
			CLUT::Half => bin_data.extend(CLUT_HALF.to_le_bytes()),
			CLUT::Full => bin_data.extend(CLUT_FULL.to_le_bytes()),
		}

		// Bit depth
		bin_data.extend(self.bit_depth.to_le_bytes());

		// Width, height
		bin_data.extend(self.width.to_le_bytes());
		bin_data.extend(self.height.to_le_bytes());

		// Allocated texture width, height
		bin_data.extend(self.texture_width.to_le_bytes());
		bin_data.extend(self.texture_height.to_le_bytes());

		// Hash
		bin_data.extend(self.hash.to_le_bytes());

		// Palette
		bin_data.extend(self.get_palette());

		// Pixel data
		match self.mode {
			Mode::Palette	=> (),
			Mode::Raw		=> bin_data.extend(self.get_pixels()),
			_				=> bin_data.extend(self.compress_acpr()),
		}

		return bin_data;
	}


}


impl Deserialization for BinSprite {
	// Assumes data has been pre-identified as a Sprite
	fn deserialize(bin_data: Vec<u8>) -> Gd<BinSprite> {
		let mode: Mode;
		let clut: CLUT;
		let bit_depth: u16;
		let width: u16;
		let height: u16;
		let mut texture_width: u16 = 0;
		let mut texture_height: u16 = 0;
		let hash: u16;
		let palette: Vec<u8>;
		let pixels: Vec<u8>;

		let ggxp_compressed: bool;
		let mut pal_size: usize = 0;

		// GGX Plus sprites
		if GGXP_MODES.contains(&bin_data[ADDRESS_MODE]) {
			mode = Mode::GGXP;

			match bin_data[ADDRESS_MODE] {
				MODE_GGXP4 => bit_depth = DEPTH_4,
				MODE_GGXP8 => bit_depth = DEPTH_8,
				_ => panic!("bin_sprite.rs::deserialize() -> Invalid GGX Plus mode!"),
			}

			// High nibble - compression value
			match bin_data[ADDRESS_GGXP_CC] >> 4 {
				GGXP_UNCOMPRESSED => ggxp_compressed = false,
				GGXP_COMPRESSED => ggxp_compressed = true,
				_ => panic!("bin_sprite.rs::deserialize() -> Invalid GGX Plus compression!"),
			}

			// Low nibble - CLUT value
			match bin_data[ADDRESS_GGXP_CC] & 0xF {
				GGXP_CLUT_NONE => clut = CLUT::None,
				GGXP_CLUT_HALF => clut = CLUT::Half,
				GGXP_CLUT_FULL => clut = CLUT::Full,
				_ => panic!("bin_sprite.rs::deserialize() -> Invalid GGX Plus CLUT!"),
			}

			width = u16::from_le_bytes([
				bin_data[ADDRESS_GGXP_WIDTH + 0],
				bin_data[ADDRESS_GGXP_WIDTH + 1],
			]);

			height = u16::from_le_bytes([
				bin_data[ADDRESS_GGXP_HEIGHT + 0],
				bin_data[ADDRESS_GGXP_HEIGHT + 1],
			]);

			// Texture size
			let mut p2_width: u16 = min(width.next_power_of_two(), 512);
			let mut p2_height: u16 = min(height.next_power_of_two(), 512);

			while p2_width > 1 {
				p2_width >>= 1;
				texture_width += 1;
			}

			while p2_height > 1 {
				p2_height >>= 1;
				texture_height += 1;
			}

			// Hash, palette
			pal_size = 4 * (2usize.pow(bit_depth as u32));

			match clut {
				CLUT::None => pal_size = 0,
				CLUT::Half => pal_size /= 2,
				CLUT::Full => (),
			}

			hash = generate_hash(bin_data[ADDRESS_HEADER_END + pal_size..].to_vec());
		} else {
			// Mode
			match u16::from_le_bytes([
				bin_data[ADDRESS_MODE + 0],
				bin_data[ADDRESS_MODE + 1],
			]) {
				MODE_RAW => mode = Mode::Raw,
				MODE_ACPR => mode = Mode::ACPR,
				MODE_PALETTE => mode = Mode::Palette,
				MODE_5 => mode = Mode::Mode5,
				_ => panic!("bin_sprite.rs::deserialize() -> Invalid mode!"),
			}

			// Bit depth
			bit_depth = u16::from_le_bytes([
				bin_data[ADDRESS_DEPTH + 0],
				bin_data[ADDRESS_DEPTH + 1],
			]);

			// CLUT
			match u16::from_le_bytes([
				bin_data[ADDRESS_CLUT + 0],
				bin_data[ADDRESS_CLUT + 1],
			]) {
				CLUT_NONE => clut = CLUT::None,
				CLUT_HALF => clut = CLUT::Half,
				CLUT_FULL => clut = CLUT::Full,
				_ => panic!("bin_sprite.rs::deserialize() -> Invalid CLUT!"),
			}

			width = u16::from_le_bytes([
				bin_data[ADDRESS_WIDTH + 0],
				bin_data[ADDRESS_WIDTH + 1],
			]);

			height = u16::from_le_bytes([
				bin_data[ADDRESS_HEIGHT + 0],
				bin_data[ADDRESS_HEIGHT + 1],
			]);

			texture_width = u16::from_le_bytes([
				bin_data[ADDRESS_TEX_WIDTH + 0],
				bin_data[ADDRESS_TEX_WIDTH + 1],
			]);

			texture_height = u16::from_le_bytes([
				bin_data[ADDRESS_TEX_HEIGHT + 0],
				bin_data[ADDRESS_TEX_HEIGHT + 1],
			]);

			hash = u16::from_le_bytes([
				bin_data[ADDRESS_HASH + 0],
				bin_data[ADDRESS_HASH + 1],
			]);

			pal_size = 4 * 2usize.pow(bit_depth as u32);

			match clut {
				CLUT::None => pal_size = 0,
				CLUT::Half => pal_size /= 2,
				CLUT::Full => (),
			}
		}

		palette = bin_data[ADDRESS_HEADER_END..ADDRESS_HEADER_END + pal_size].to_vec();
		let pointer: usize = ADDRESS_HEADER_END + pal_size;
		let pixel_data: Vec<u8> = bin_data[pointer..].to_vec();

		match mode {
			Mode::Palette => pixels = Vec::new(),
			Mode::Raw => pixels = pixel_data,
			Mode::ACPR => pixels = Self::decompress_acpr(pixel_data, width, height, bit_depth),
			Mode::Mode5 => pixels = Self::decompress_mode5(pixel_data),
			Mode::GGXP => pixels = Self::decompress_ggx(pixel_data, width, height, bit_depth),
		}

		return Gd::from_init_fn(|base| {
			Self {
				base,
				mode,
				clut,
				bit_depth,
				width,
				height,
				texture_width,
				texture_height,
				hash,
				manual_hash: false,
				palette,
				pixels,
			}
		});
	}
}


pub fn extract_bits(chunk: &[u8]) -> VecDeque<bool> {
	let mut new_chunk: VecDeque<bool> = VecDeque::new();
	let mut byte: u8;

	for pointer in 0..chunk.len() {
		byte = chunk[pointer];
		for _i in 0..8 {
			new_chunk.push_back((byte & 1) == 1);
			byte >>= 1;
		}
	}

	return new_chunk;
}


pub fn pop_bits(chunk: &mut VecDeque<bool>, bit_count: usize) -> u8 {
	let mut byte: u8 = 0;

	for bit in 0..bit_count {
		match chunk.pop_front() {
			Some(true) => byte |= 1 << bit,
			Some(false) => (),
			None => break,
		}
	}

	return byte;
}


#[godot_api]
impl BinSprite {
	pub fn get_palette(&self) -> Vec<u8> {
		// Guard rail
		match self.clut {
			CLUT::None => Vec::new(),
			_ => self.palette.clone(),
		}
	}


	pub fn get_pixels(&self) -> Vec<u8> {
		return self.pixels.clone();
	}


	/// Reindexing function. Reorders colors from 1-2-3-4 to 1-3-2-4 and vice-versa.
	#[func]
	pub fn reindex(&mut self) {
		self.pixels = sprite_transform::reindex_vector(self.get_pixels());
	}


	pub fn compress(&self) -> Vec<u8> {
		// Output is +R-only for now. If other formats get support, this will change.
		return self.compress_acpr();
	}


	pub fn compress_acpr(&self) -> Vec<u8> {
		const WINDOW_SIZE: usize = 512;
		const TOKEN_SIZE_MAX: usize = 130;

		let pixels: Vec<u8>;

		match self.bit_depth {
			DEPTH_4 => pixels = sprite_transform::bpp_to_4(self.get_pixels(), true),
			_ => pixels = self.get_pixels(),
		}

		// Loop variables
		let mut current_pixel: usize = 0;
		let mut iterations: u32 = 0;

		// Output bit stream
		let mut compressed_stream: Vec<u8> = Vec::new();
		let mut bit_writer = BitWriter::endian(&mut compressed_stream, BigEndian);

		// Iterate vector
		while current_pixel < pixels.len() {
			// Token window origin point
			let window_origin: usize;

			if current_pixel > WINDOW_SIZE {
				window_origin = current_pixel - WINDOW_SIZE;
			} else {
				window_origin = 0;
			}

			if current_pixel >= 4 && pixels.len() - current_pixel > 2 {
				let mut best_sequence_offset: usize = 0;
				let mut best_sequence_length: usize = 0;
				let mut token_size_max_local: usize = min(TOKEN_SIZE_MAX, current_pixel);
				token_size_max_local = min(token_size_max_local, pixels.len() - current_pixel);

				// New window scan, slower, better compression (matches game's)
				for window_offset in 0..510 {
					let mut sequence_length: usize = 0;

					while sequence_length < token_size_max_local {
						let window_index: usize = window_origin + window_offset + sequence_length;

						if window_index >= current_pixel {
							break;
						}

						if pixels[current_pixel + sequence_length] == pixels[window_index] {
							sequence_length += 1;
						} else {
							break;
						}
					}

					if sequence_length > best_sequence_length {
						best_sequence_length = sequence_length;
						best_sequence_offset = window_offset;
					}

					if sequence_length >= token_size_max_local {
						break;
					}
				}

				if best_sequence_length > 2 {
					let _ = bit_writer.write_bit(false);
					let _ = bit_writer.write(9, best_sequence_offset as u16);
					let _ = bit_writer.write(7, (best_sequence_length as u8) - 3);
					current_pixel += best_sequence_length;
					iterations += 1;
					continue;
				}
			}

			// Literal indicator
			let _ = bit_writer.write_bit(true);

			// Pixels
			let _ = bit_writer.write(8, pixels[current_pixel]);

			if current_pixel + 1 < pixels.len() {
				let _ = bit_writer.write(8, pixels[current_pixel + 1]);
			} else {
				let _ = bit_writer.write(8, 0u8);
			}

			// Increment position
			current_pixel += 2;
			iterations += 1;
		}

		// Pad and close bit stream
		bit_writer.byte_align().expect(
			"main::make_compressed_sprite() error: Could not align bitstream");
		bit_writer.into_writer();

		let file_byte_length: usize = compressed_stream.len() + 20;

		if file_byte_length % 16 != 0 {
			for _i in 0..(16 - file_byte_length % 16) {
				compressed_stream.push(0xFF);
			}
		}

		let mut bin_data: Vec<u8> = Vec::new();

		// Write iterations
		bin_data.extend([
			(iterations >> 16) as u8,
			(iterations >> 24) as u8,
			(iterations >> 00) as u8,
			(iterations >> 08) as u8,
		]);

		// Write compressed data
		for byte in 0..compressed_stream.len() / 2 {
			bin_data.extend([
				compressed_stream[2 * byte + 1],
				compressed_stream[2 + byte + 0],
			])
		}

		return bin_data;
	}


	pub fn decompress_acpr(bin_data: Vec<u8>, width: u16, height: u16, bit_depth: u16) -> Vec<u8> {
		let pixel_count: usize = width as usize * height as usize;
		let mut pointer: usize = 0x00;

		// Read iterations
		let iterations: u32 = u32::from_le_bytes([
			bin_data[pointer + 0x02],
			bin_data[pointer + 0x03],
			bin_data[pointer + 0x00],
			bin_data[pointer + 0x01]
		]);

		// Move pointer past iterations
		pointer += 0x04;

		// Get byte data
		let mut byte_data: Vec<u8> = Vec::with_capacity(bin_data.len() - pointer);
		while pointer + 1 < bin_data.len() {
			byte_data.push(bin_data[pointer + 1]);
			byte_data.push(bin_data[pointer]);
			pointer += 2;
		}

		// Read as bit stream
		let mut bit_reader = BitReader::endian(Cursor::new(&byte_data), BigEndian);

		// Pixel vector
		let mut pixel_vector: Vec<u8> = Vec::new();

		for _i in 0..iterations {
			// Literal mode
			if bit_reader.read_bit().unwrap() == true {
				pixel_vector.push(bit_reader.read(8).unwrap());

				// Stray byte guard rail
				if pixel_vector.len() + 1 < pixel_count {
					pixel_vector.push(bit_reader.read(8).unwrap());
				}
			}

			// Token mode
			else {
				let mut window_origin: usize = 0;
				if pixel_vector.len() > 512 {
					window_origin = pixel_vector.len() - 512;
				}

				let offset: usize = bit_reader.read::<u16>(9).unwrap() as usize;
				let length: usize = 3 + bit_reader.read::<u8>(7).unwrap() as usize;

				for pixel in 0..length {
					pixel_vector.push(pixel_vector[window_origin + offset + pixel]);
				}
			}
		}

		// Bit depth management
		match bit_depth {
			4 => pixel_vector = sprite_transform::bpp_from_4(pixel_vector, true),
			8 => (),
			// Shouldn't ever happen
			_ => panic!("sprite_compress::decompress() error: Invalid BIN bit depth"),
		}

		pixel_vector.resize(width as usize * height as usize, 0u8);
		return pixel_vector;
	}


	pub fn decompress_mode5(bin_data: Vec<u8>) -> Vec<u8> {
		// Read secondary header
		let width: usize = u16::from_le_bytes([
			bin_data[0x0],
			bin_data[0x1],
		]) as usize;

		let mut height: usize = u16::from_le_bytes([
			bin_data[0x2],
			bin_data[0x3],
		]) as usize;

		// 0x14, 0x15: bit depth

		// There are apparently modes 5-4 and 5-5
		let mode: u16 = u16::from_le_bytes([
			bin_data[0x6],
			bin_data[0x7],
		]);

		// Data chunks
		let from_a: usize = u16::from_le_bytes([
			bin_data[0x8],
			bin_data[0x9],
		]) as usize * 0x08;

		let from_b: usize = u16::from_le_bytes([
			bin_data[0xA],
			bin_data[0xB],
		]) as usize * 0x08;

		let from_c: usize = u16::from_le_bytes([
			bin_data[0xC],
			bin_data[0xD],
		]) as usize * 0x08;

		let from_d: usize = u16::from_le_bytes([
			bin_data[0xE],
			bin_data[0xF],
		]) as usize * 0x08;

		let mut chunk_a: VecDeque<bool> = extract_bits(&bin_data[from_a..from_b]);
		let mut chunk_b: VecDeque<bool> = extract_bits(&bin_data[from_b..from_c]);
		let mut chunk_c: VecDeque<u8> = VecDeque::new();
		let mut chunk_d: Vec<u8> = Vec::new(); // 1 byte at a time
		chunk_d.extend_from_slice(&bin_data[from_d..]);

		// 12*5 bits, then skip 4 bits
		let mut chunk_c_raw: Vec<u8> = Vec::new();
		chunk_c_raw.extend_from_slice(&bin_data[from_c..from_d]);

		{	// Save myself some chunk-C-based headache
			for i in 0..chunk_c_raw.len() / 8 {
				let mut qword: u64 = u64::from_le_bytes([
					chunk_c_raw[8 * i + 0], chunk_c_raw[8 * i + 1],
					chunk_c_raw[8 * i + 2], chunk_c_raw[8 * i + 3],
					chunk_c_raw[8 * i + 4], chunk_c_raw[8 * i + 5],
					chunk_c_raw[8 * i + 6], chunk_c_raw[8 * i + 7],
				]);

				for _j in 0..12 {
					chunk_c.push_back((qword & 0x1F) as u8);
					qword >>= 5;
				}
			}
		}

		let mut pointer_d: usize = 0;

		let pixel_count: usize = width * height;
		let mut pixel_vector: Vec<u8> = Vec::with_capacity(pixel_count);
		pixel_vector.resize(pixel_count, 0);

		let mut pointer_write: usize = 0;

		height /= 2;

		let mut iterations: u16 = 0;
		let mut cache_1: u8 = 0;
		let mut cache_2: u8 = 0;
		let mut pixel_a: u8 = 0;
		let mut pixel_b: u8 = 0;
		let mut pixel_c: u8 = 0;
		let mut pixel_d: u8 = 0;

		if mode == 5 {
			for _y in 0..height {
				for _x in 0..width / 2 {
					if iterations == 0 {
						if chunk_a.pop_front().unwrap() {
							if chunk_a.pop_front().unwrap() {
								// Top line
								pixel_a = chunk_c.pop_front().unwrap();
								cache_1 = chunk_c.pop_front().unwrap();
								pixel_b = cache_1;

								// Bottom line
								pixel_c = chunk_c.pop_front().unwrap();
								cache_2 = chunk_c.pop_front().unwrap();
								pixel_d = cache_2;
							}

							else if chunk_a.pop_front().unwrap() {
								iterations = chunk_d[pointer_d] as u16 + 3;
								pointer_d += 1;
							}
						}

						else {
							if chunk_a.pop_front().unwrap() {
								if chunk_a.pop_front().unwrap() {
									cache_1 = chunk_c.pop_front().unwrap();
									pixel_a = cache_1;
								}

								else {
									if chunk_a.pop_front().unwrap() {
										pixel_a = cache_2;
									}
									else {
										pixel_a = cache_1;
									}
								}

								pixel_b = pixel_a;
								pixel_c = pixel_a;
								pixel_d = pixel_a;
							}

							else {
								if chunk_a.pop_front().unwrap() {
									cache_1 = chunk_c.pop_front().unwrap();
								}

								if chunk_a.pop_front().unwrap() {
									cache_2 = chunk_c.pop_front().unwrap();
								}

								if chunk_b.pop_front().unwrap() {
									pixel_d = cache_2;
								} else {
									pixel_d = cache_1;
								}

								if chunk_b.pop_front().unwrap() {
									pixel_c = cache_2;
								} else {
									pixel_c = cache_1;
								}

								if chunk_b.pop_front().unwrap() {
									pixel_b = cache_2;
								} else {
									pixel_b = cache_1;
								}

								if chunk_b.pop_front().unwrap() {
									pixel_a = cache_2;
								} else {
									pixel_a = cache_1;
								}
							}
						}
					}

					else {
						iterations -= 1;
					}

					pixel_vector[pointer_write + 0] = pixel_a;
					pixel_vector[pointer_write + 1] = pixel_b;
					pixel_vector[pointer_write + width + 0] = pixel_c;
					pixel_vector[pointer_write + width + 1] = pixel_d;
					pointer_write += 2;
				}
				pointer_write += width;
			}
		}

		if mode == 4 {
			for _y in 0..height {
				for _x in 0..width / 2 {
					if iterations == 0 {
						if chunk_a.pop_front().unwrap() {
							if chunk_a.pop_front().unwrap() {
								pixel_a = pop_bits(&mut chunk_b, 4);
								cache_1 = pop_bits(&mut chunk_b, 4);
								pixel_c = pop_bits(&mut chunk_b, 4);
								cache_2 = pop_bits(&mut chunk_b, 4);

								pixel_b = cache_1;
								pixel_d = cache_2;
							}

							else if chunk_a.pop_front().unwrap() {
								iterations = chunk_d[pointer_d] as u16 + 3;
								pointer_d += 1;
							}
						}

						else if chunk_a.pop_front().unwrap() {
							if chunk_a.pop_front().unwrap() {
								cache_1 = pop_bits(&mut chunk_b, 4);
								pixel_c = cache_1;
							}

							else if chunk_a.pop_front().unwrap() {
								pixel_c = cache_2;
							} else {
								pixel_c = cache_1;
							}

							pixel_a = pixel_c;
							pixel_b = pixel_c;
							pixel_d = pixel_c;
						}

						else {
							if chunk_a.pop_front().unwrap() {
								cache_1 = pop_bits(&mut chunk_b, 4);
							}
							if chunk_a.pop_front().unwrap() {
								cache_2 = pop_bits(&mut chunk_b, 4);
							}

							if chunk_b.pop_front().unwrap() {
								pixel_d = cache_2;
							} else {
								pixel_d = cache_1;
							}

							if chunk_b.pop_front().unwrap() {
								pixel_c = cache_2;
							} else {
								pixel_c = cache_1;
							}

							if chunk_b.pop_front().unwrap() {
								pixel_b = cache_2;
							} else {
								pixel_b = cache_1;
							}

							if chunk_b.pop_front().unwrap() {
								pixel_a = cache_2;
							} else {
								pixel_a = cache_1;
							}
						}
					}

					else {
						iterations -= 1;
					}

					pixel_vector[pointer_write + 0] = pixel_a;
					pixel_vector[pointer_write + 1] = pixel_b;
					pixel_vector[pointer_write + width + 0] = pixel_c;
					pixel_vector[pointer_write + width + 1] = pixel_d;
					pointer_write += 2;
				}
				pointer_write += width;
			}
		}

		return pixel_vector;
	}


	pub fn decompress_ggx(bin_data: Vec<u8>, width: u16, height: u16, bit_depth: u16) -> Vec<u8> {
		let mut pointer: usize = 0x00;
		let mut pixel_vector: Vec<u8> = Vec::new();

		while pixel_vector.len() < width as usize * height as usize {
			// Literals
			if bin_data[pointer] & 0xC0 == 0 {
				for _i in 0..bin_data[pointer] as usize + 1 {
					pointer += 0x01;

					match bit_depth {
						4 => {
							pixel_vector.push(bin_data[pointer] & 0xF);
							pixel_vector.push(bin_data[pointer] >> 4);
						},

						_ => {
							pixel_vector.push(bin_data[pointer]);
						}
					}
				}
			}

			// Tokens
			else {
				let mut token_count: usize = (bin_data[pointer] as usize + 0xC3) & 0xFF;
				if bit_depth == 4 {
					token_count *= 2;
				}

				for _i in 0..token_count {
					pixel_vector.push(pixel_vector[pixel_vector.len() - 1]);
				}
			}

			// Next byte
			pointer += 0x01;
		}
		return pixel_vector;
	}
}