use crate::Identification;
use crate::Serialization;
use crate::Deserialization;

use std::cmp::min;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use bmp_rust::bmp::{BITMAPFILEHEADER, BMP, DIBHEADER};
use godot::prelude::*;
use crate::sprite_transform;

use color_quant::NeuQuant;

#[derive(Clone)]
enum Mode {
	Raw,
	ACPR,
	Palette,
	Mode5,
	GGXP,
}

#[derive(Clone)]
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

const ADDRESS_GGXP_CC		: usize = 0x01;	// 'C'ompression and 'C'LUT address for GGX Plus sprites
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

const COLOR_COUNT_4_HALF: usize = 8;
const COLOR_COUNT_4_FULL: usize = 16;
const COLOR_COUNT_8_HALF: usize = 128;
const COLOR_COUNT_8_FULL: usize = 256;

const CLUT_SIZE_4_HALF	: usize = 4 * COLOR_COUNT_4_HALF;
const CLUT_SIZE_4_FULL	: usize = 4 * COLOR_COUNT_4_FULL;
const CLUT_SIZE_8_HALF	: usize = 4 * COLOR_COUNT_8_HALF;
const CLUT_SIZE_8_FULL	: usize = 4 * COLOR_COUNT_8_FULL;

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

// BMP files
const BITMAPCOREHEADER_SIZE: usize = 12;
const BMP_COLOR_24: usize = 3;
const BMP_COLOR_32: usize = 4;

// Quantization parameters
const QUANT_DEFAULT_QUALITY: i32 = 10;


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
impl BinSprite {
	// MODE
	pub fn get_mode(&self) -> &Mode {
		return &self.mode;
	}
	
	
	pub fn set_mode(&mut self, mode: u16) {
		match mode {
			MODE_RAW => self.mode = Mode::Raw,
			MODE_ACPR => self.mode = Mode::ACPR,
			MODE_PALETTE => self.mode = Mode::Palette,
			MODE_5 => self.mode = Mode::Mode5,
			_ => self.mode = Mode::GGXP,
		}
	}


	// PALETTE
	pub fn has_palette(&self) -> bool {
		match self.clut {
			CLUT::None => return false,
			_ => if self.palette.is_empty() { return false }
		}

		return true
	}


	pub fn get_palette(&self) -> Vec<u8> {
		// Guard rail
		match self.clut {
			CLUT::None => Vec::new(),
			_ => self.palette.clone(),
		}
	}


	pub fn set_palette(&mut self, mut new_palette: Vec<u8>) {
		new_palette.resize(4 * self.get_color_count(), 0u8);
		self.palette = new_palette;
	}


	pub fn get_color_count(&self) -> usize {
		match self.clut {
			CLUT::None => return 0,

			CLUT::Half => {
				if self.get_bit_depth() == 4 {
					return COLOR_COUNT_4_HALF;
				} else {
					return COLOR_COUNT_8_HALF;
				}
			},

			CLUT::Full => {
				if self.get_bit_depth() == 4 {
					return COLOR_COUNT_4_FULL;
				} else {
					return COLOR_COUNT_8_FULL;
				}
			}
		}
	}


	pub fn get_color(&self, index: usize) -> (u8, u8, u8, u8) {
		if index >= self.get_color_count() {
			return (0, 0, 0, 0);
		} else {
			return (
				self.palette[4 * index + 0],
				self.palette[4 * index + 1],
				self.palette[4 * index + 2],
				self.palette[4 * index + 3],
			);
		}
	}


	pub fn set_color(&mut self, index: usize, color: (u8, u8, u8, u8)) {
		if index >= self.get_color_count() {
			return;
		}

		(
			self.palette[4 * index + 0],
			self.palette[4 * index + 1],
			self.palette[4 * index + 2],
			self.palette[4 * index + 3],
		) = color;
	}


	pub fn purge_palette(&mut self) {
		self.palette = vec![];
		self.clut = CLUT::None;
	}


	pub fn palette_halve_alpha(&mut self) {
		for index in 0..self.get_color_count() {
			let mut color = self.get_color(index);

			if color.3 == 0xFF {
				color.3 = 0x80;
			} else {
				color.3 /= 2;
			}

			self.set_color(index, color);
		}
	}


	pub fn palette_double_alpha(&mut self) {
		for index in 0..self.get_color_count() {
			let mut color = self.get_color(index);

			if color.3 >= 0x80 {
				color.3 = 0xFF;
			} else {
				color.3 *= 2;
			}

			self.set_color(index, color);
		}
	}


	pub fn palette_make_opaque(&mut self) {
		for index in 0..self.get_color_count() {
			let mut color = self.get_color(index);
			color.3 = 0xFF;
			self.set_color(index, color);
		}
	}


	// BIT DEPTH
	pub fn get_bit_depth(&self) -> u16 {
		if self.bit_depth == DEPTH_4
		{ return DEPTH_4; }
		else
		{ return DEPTH_8; }
	}


	pub fn set_bit_depth_4(&mut self) {
		self.bit_depth = DEPTH_4;
	}

	
	pub fn set_bit_depth_8(&mut self) {
		self.bit_depth = DEPTH_8;
	}

	
	// DIMENSIONS
	pub fn get_width(&self) -> u16 {
		return self.width;
	}
	
	
	pub fn get_height(&self) -> u16 {
		return self.height;
	}
	

	// PIXEL VECTOR
	pub fn get_pixels(&self) -> Vec<u8> {
		return self.pixels.clone();
	}


	pub fn get_pixel(&self, x: u16, y: u16) -> u8 {
		if x > self.width || y > self.height {
			return 0;
		}

		return self.pixels[(y * self.width + x) as usize];
	}


	pub fn set_pixels(&mut self, new_pixels: Vec<u8>) {
		self.pixels = new_pixels;
	}


	// TRANSFORMS
	pub fn flip_h(&mut self) {
		let mut output: Vec<u8> = Vec::with_capacity(self.pixels.len());

		for y in 0..self.height {
			for x in 0..self.width {
				output.push(self.get_pixel(self.width - x - 1, y));
			}
		}

		self.pixels = output;
	}


	pub fn flip_v(&mut self) {
		let mut output: Vec<u8> = Vec::with_capacity(self.pixels.len());

		let w: usize = self.width as usize;
		let h: usize = self.height as usize;

		for y in 0..h {
			let pointer: usize = (h - y - 1) * w;
			output.extend_from_slice(&self.pixels[pointer..pointer + w]);
		}

		self.pixels = output;
	}


	/// Pixel reindexing function. Reorders colors from 1-2-3-4 to 1-3-2-4 and vice-versa.
	#[func]
	pub fn reindex_pixels(&mut self) {
		if self.get_bit_depth() == 4 { return; }
		self.pixels = sprite_transform::reindex_vector(self.get_pixels());
	}


	/// Palette reindexing function. Same as above, but by rearranging the palette, rather than each
	/// pixel's color index.
	#[func]
	pub fn reindex_palette(&mut self) {
		if self.get_bit_depth() == 4 { return; }
		self.palette = sprite_transform::reindex_rgba_vector(self.get_palette());
	}


	// COMPRESSION
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
	
	
	// UTILS
	pub fn clone(&self) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			BinSprite {
				base,
				mode: self.mode.clone(),
				clut: self.clut.clone(),
				bit_depth: self.get_bit_depth(),
				width: self.width,
				height: self.height,
				texture_width: self.texture_width,
				texture_height: self.texture_height,
				hash: self.hash,
				manual_hash: self.manual_hash,
				palette: self.get_palette(),
				pixels: self.get_pixels(),
			}
		});
	}
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
		match self.clut {
			CLUT::None => (),
			_ => bin_data.extend(self.get_palette()),
		}

		// Pixel data
		match self.mode {
			Mode::Raw		=> bin_data.extend(self.get_pixels()),
			Mode::Palette	=> (),
			_				=> bin_data.extend(self.compress_acpr()),
		}

		return bin_data;
	}
}


impl Deserialization for BinSprite {
	fn deserialize(bin_data: &Vec<u8>) -> Option<Gd<BinSprite>> {
		if !Self::identify(&bin_data) {
			return None;
		}
		
		let mode: Mode;
		let clut: CLUT;
		let bit_depth: u16;
		let width: u16;
		let height: u16;
		let texture_width: u16;
		let texture_height: u16;
		let hash: u16;
		let palette: Vec<u8>;
		let mut pixels: Vec<u8>;

		let mut ggxp_compressed: bool = false;
		let mut pal_size: usize;

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
			texture_width = get_texture_size(width);
			texture_height = get_texture_size(height);

			// Hash, palette
			pal_size = 4 * (2usize.pow(bit_depth as u32));

			match clut {
				CLUT::None => pal_size = 0,
				CLUT::Half => pal_size /= 2,
				CLUT::Full => (),
			}

			hash = generate_hash(&bin_data[ADDRESS_HEADER_END + pal_size..].to_vec());
		}

		// The rest
		else {
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
			Mode::ACPR => pixels = decompress_acpr(bin_data),
			Mode::Mode5 => pixels = decompress_mode5(bin_data),
			Mode::GGXP => {
				if ggxp_compressed { pixels = decompress_ggx(bin_data); }
				else { pixels = pixel_data; }
			},
		}

		// Finishing touches
		//pixels = sprite_transform::trim_padding(pixels, width as usize, height as usize, false);

		return Some(Gd::from_init_fn(|base| {
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
		}));
	}
}


// CREATION ========================================================================================


pub fn init_empty_palette() -> Gd<BinSprite> {
	return Gd::from_init_fn(|base| {
		BinSprite {
			base,
			mode: Mode::Palette,
			clut: CLUT::Full,
			bit_depth: DEPTH_8,
			width: 0,
			height: 0,
			texture_width: 0,
			texture_height: 0,
			hash: 0,
			manual_hash: false,
			palette: vec![0u8; CLUT_SIZE_8_FULL],
			pixels: vec![],
		}
	})
}


// GENERIC UTILS ===================================================================================


// Texture size
pub fn get_texture_size(dimension: u16) -> u16 {
	let mut p2_dimension: u16 = min(dimension.next_power_of_two(), 512);
	let mut texture_size: u16 = 0;

	while p2_dimension > 1 {
		p2_dimension >>= 1;
		texture_size += 1;
	}

	return texture_size;
}


// Generic
pub fn generate_hash(bin_data: &Vec<u8>) -> u16 {
	let mut hash: u16 = 0;

	for byte in 0..bin_data.len() / 2 {
		hash = hash ^ u16::from_le_bytes([
			bin_data[byte + 0],
			bin_data[byte + 1],
		]);
	}

	return hash;
}


// Palletization
pub fn quantize(rgba: Vec<u8>, bit_depth: u16, quality_level: i32) -> (Vec<u8>, Vec<u8>) {
	const QUALITY_LEVEL: i32 = 10;

	let color_count: usize;
	let palette: Vec<u8>;
	let mut pixel_vector: Vec<u8> = Vec::new();

	if bit_depth == DEPTH_4 {
		color_count = COLOR_COUNT_4_FULL;
	} else {
		color_count = COLOR_COUNT_8_FULL;
	}

	let quant: NeuQuant = NeuQuant::new(quality_level, color_count, rgba.as_slice());
	palette = quant.color_map_rgba();

	for pixel in 0..rgba.len() / 4 {
		pixel_vector.push(
			quant.index_of(&[
				rgba[4 * pixel + 0],
				rgba[4 * pixel + 1],
				rgba[4 * pixel + 2],
				rgba[4 * pixel + 3],
			]) as u8
		);
	}

	return (palette, pixel_vector);
}


// DECOMPRESSION ===================================================================================


// Mode 5 auxiliaries
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


// Mode 1
pub fn decompress_acpr(bin_data: &Vec<u8>) -> Vec<u8> {
	let bit_depth: u16 = u16::from_le_bytes([
		bin_data[ADDRESS_DEPTH + 0],
		bin_data[ADDRESS_DEPTH + 1],
	]);

	let width: usize = u16::from_le_bytes([
		bin_data[ADDRESS_WIDTH + 0],
		bin_data[ADDRESS_WIDTH + 1],
	]) as usize;

	let height: usize = u16::from_le_bytes([
		bin_data[ADDRESS_HEIGHT + 0],
		bin_data[ADDRESS_HEIGHT + 1],
	]) as usize;

	let clut: u16 = u16::from_le_bytes([
		bin_data[ADDRESS_CLUT + 0],
		bin_data[ADDRESS_CLUT + 1],
	]);

	let pal_size: usize;

	match clut {
		CLUT_NONE => pal_size = 0,

		CLUT_HALF => {
			if bit_depth == DEPTH_4 {
				pal_size = CLUT_SIZE_4_HALF;
			} else {
				pal_size = CLUT_SIZE_8_HALF;
			}
		},

		CLUT_FULL => {
			if bit_depth == DEPTH_4 {
				pal_size = CLUT_SIZE_4_FULL;
			} else {
				pal_size = CLUT_SIZE_8_FULL;
			}
		},

		_ => panic!("bin_sprite.rs::decompress_acpr() -> Invalid CLUT!"),
	}

	let pixel_count: usize = width * height;
	let mut pointer: usize = ADDRESS_HEADER_END + pal_size;

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

	pixel_vector.resize(width * height, 0u8);
	return pixel_vector;
}


// Mode 5
pub fn decompress_mode5(bin_data: &Vec<u8>) -> Vec<u8> {
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


// Modes 19 and 20 (0x13 and 0x14)
pub fn decompress_ggx(bin_data: &Vec<u8>) -> Vec<u8> {
	let width: usize = u16::from_le_bytes([
		bin_data[ADDRESS_GGXP_WIDTH + 0],
		bin_data[ADDRESS_GGXP_WIDTH + 1],
	]) as usize;

	let height: usize = u16::from_le_bytes([
		bin_data[ADDRESS_GGXP_HEIGHT + 0],
		bin_data[ADDRESS_GGXP_HEIGHT + 1],
	]) as usize;

	let bit_depth: u16;

	match bin_data[ADDRESS_MODE] {
		MODE_GGXP4 => bit_depth = DEPTH_4,
		MODE_GGXP8 => bit_depth = DEPTH_8,
		_ => panic!("bin_sprite.rs::decompress_ggx() -> Invalid bit depth!"),
	}

	let mut pointer: usize = 0x00;
	let mut pixel_vector: Vec<u8> = Vec::new();

	while pixel_vector.len() < width * height {
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
			if bit_depth == DEPTH_4 {
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


// LOAD FROM FILE ==================================================================================


pub fn load_from_file(source_file: &PathBuf, with_palette: bool) -> Option<Gd<BinSprite>> {
	match source_file.extension() {
		Some(os_str) => match os_str.to_ascii_lowercase().to_str() {
			Some("bin") => return load_from_bin(source_file, with_palette),
			Some("png") => return load_from_png(source_file, with_palette),
			Some("bmp") => return load_from_bmp(source_file, with_palette),
			Some("raw") => return load_from_raw(source_file),
			_ => {
				println!("sprite_import_export::import_sprites() error: Invalid source format provided");
				return None;
			},
		},

		_ => {
			println!("sprite_import_export::import_sprites() error: Invalid source format provided");
			return None;
		}
	}
}


pub fn load_from_bin(source_file: &PathBuf, with_palette: bool) -> Option<Gd<BinSprite>> {
	match fs::read(&source_file) {
		Ok(data) => {
			match BinSprite::deserialize(&data) {
				Some(mut sprite) => {
					if !with_palette {
						sprite.bind_mut().purge_palette();
					}

					return Some(sprite);
				}
				
				_ => return None,
			}
		},
		
		_ => return None,
	}
}


pub fn load_from_png(source_file: &PathBuf, with_palette: bool) -> Option<Gd<BinSprite>> {
	// Get info
	let file: File;
	match File::open(&source_file) {
		Ok(value) => file = value,
		_ => {
			println!("bin_sprite::get_png() error: PNG file open error");
			println!("\tSkipped: {}", &source_file.display());
			return None;
		},
	}

	let mut decoder = png::Decoder::new(file);
	decoder.set_transformations(png::Transformations::STRIP_16);
	let mut reader = decoder.read_info().unwrap();

	let mut bit_depth: u16 = 8;
	match reader.info().bit_depth {
		png::BitDepth::One => bit_depth = 1,
		png::BitDepth::Two => bit_depth = 2,
		png::BitDepth::Four => bit_depth = DEPTH_4,
		_ => (),
	}

	let mut palette: Vec<u8> = Vec::new();

	// Get bytes
	let mut buffer = vec![0; reader.output_buffer_size()];
	let frame = reader.next_frame(&mut buffer).unwrap();

	let source_bytes: Vec<u8> = buffer[..frame.buffer_size()].to_vec();
	let mut pixel_vector: Vec<u8> = Vec::new();

	// Transfer color indices to pixel_vector
	match reader.info().color_type {
		png::ColorType::Grayscale => {
			if with_palette {
				let max: usize;

				if bit_depth == DEPTH_8 {
					max = COLOR_COUNT_8_FULL;
				} else {
					max = COLOR_COUNT_4_FULL;
				}

				for i in 0..max {
					palette.extend_from_slice(&[i as u8, i as u8, i as u8, 0xFF]);
				}
			}

			pixel_vector = source_bytes;
		},

		png::ColorType::Indexed => {
			if with_palette
			{
				match &reader.info().palette {
					Some(pal_data) => {
						let temp_pal: Vec<u8> = pal_data.to_vec();
						let color_count: usize = temp_pal.len() / 3;
						let mut alpha_vec: Vec<u8> = Vec::new();

						match &reader.info().trns {
							Some(alpha) => alpha_vec = alpha.to_vec(),
							_ => (),
						}

						alpha_vec.resize(color_count, 0x80);
						palette = vec![0; color_count * 4];

						for index in 0..color_count {
							palette[4 * index + 0] = temp_pal[3 * index + 0];
							palette[4 * index + 1] = temp_pal[3 * index + 1];
							palette[4 * index + 2] = temp_pal[3 * index + 2];
							palette[4 * index + 3] = alpha_vec[index];
						}
					}

					_ => (),
				}
			}

			pixel_vector = source_bytes;
		}

		png::ColorType::GrayscaleAlpha => {
			if with_palette {
				let max: usize;

				if bit_depth == DEPTH_8 {
					max = COLOR_COUNT_8_FULL;
				} else {
					max = COLOR_COUNT_4_FULL;
				}

				for i in 0..max {
					palette.extend_from_slice(&[i as u8, i as u8, i as u8, 0xFF]);
				}

				for pixel in 0..source_bytes.len() / 2 {
					let index: usize = source_bytes[2 * pixel + 0] as usize;
					let alpha: u8 = source_bytes[2 + pixel + 1];
					palette[index] = alpha;
				}
			}
			else {
				println!("Note: PNG has color type grayscale with alpha, will discard alpha");
				println!("\tFile: {}", &source_file.display());
			}

			for pixel in 0..source_bytes.len() / 2 {
				pixel_vector.push(source_bytes[pixel * 2]);
			}
		},

		png::ColorType::Rgb => {
			if with_palette {
				let mut rgba: Vec<u8> = Vec::new();

				for pixel in 0..source_bytes.len() / 3 {
					rgba.push(source_bytes[3 * pixel + 0]);
					rgba.push(source_bytes[3 * pixel + 1]);
					rgba.push(source_bytes[3 * pixel + 2]);
					rgba.push(0xFF);
				}

				(palette, pixel_vector) = quantize(rgba, bit_depth, QUANT_DEFAULT_QUALITY);
			}

			else {
				println!("Note: PNG has color type RGB, will use red channel as grayscale");
				println!("\tFile: {}", &source_file.display());
				for pixel in 0..source_bytes.len() / 3 {
					pixel_vector.push(source_bytes[pixel * 3]);
				}
			}
		},

		png::ColorType::Rgba => {
			if with_palette {
				(palette, pixel_vector) = quantize(source_bytes, bit_depth, QUANT_DEFAULT_QUALITY);
			}

			else {
				println!("Note: PNG has color type RGBA, will use red channel as grayscale and discard alpha");
				println!("\tFile: {}", &source_file.display());
				for pixel in 0..source_bytes.len() / 4 {
					pixel_vector.push(source_bytes[pixel * 4]);
				}
			}
		},
	}

	// Bit depth management
	match bit_depth {
		1 => pixel_vector = sprite_transform::bpp_from_1(pixel_vector),
		2 => pixel_vector = sprite_transform::bpp_from_2(pixel_vector),
		4 => pixel_vector = sprite_transform::bpp_from_4(pixel_vector, false),
		_ => (),	// Hope and pray
	}

	if bit_depth < DEPTH_8
	{ bit_depth = DEPTH_4 }
	else
	{ bit_depth = DEPTH_8 }

	let clut: CLUT;

	if palette.is_empty() {
		clut = CLUT::None;
	} else {
		match bit_depth {
			DEPTH_4 => {
				if palette.len() <= CLUT_SIZE_4_FULL {
					clut = CLUT::Half;
					palette.resize(CLUT_SIZE_4_HALF, 0u8);
				} else {
					clut = CLUT::Full;
					palette.resize(CLUT_SIZE_4_FULL, 0u8);
				}
			},
			
			_ => {
				if palette.len() <= CLUT_SIZE_8_HALF {
					clut = CLUT::Half;
					palette.resize(CLUT_SIZE_8_HALF, 0u8);
				}
				else {
					clut = CLUT::Full;
					palette.resize(CLUT_SIZE_8_FULL, 0u8);
				}
			}
		}
	}

	let width: u16 = reader.info().width as u16;
	let height: u16 = reader.info().height as u16;

	return Some(Gd::from_init_fn(|base| {
		BinSprite {
			base,
			mode: Mode::ACPR,
			bit_depth,
			clut,
			width,
			height,
			texture_width: get_texture_size(width),
			texture_height: get_texture_size(height),
			hash: generate_hash(&pixel_vector),
			manual_hash: false,
			palette,
			pixels: pixel_vector,
		}
	}));
}


pub fn load_from_bmp(source_file: &PathBuf, with_palette: bool) -> Option<Gd<BinSprite>> {
	// Not using BMP::new_from_file as it does not account for
	// failing to read from a file and will panic if it does
	let mut palette: Vec<u8> = vec![];

	// File read
	let bytes: Vec<u8>;
	match fs::read(source_file) {
		Ok(value) => bytes = value,
		_ => {
			println!("bin_sprite::load_from_bmp() error: BMP file read error");
			println!("\tSkipped: {}", &source_file.display());
			return None;
		},
	}

	let mut bmp: BMP = BMP::new(50i32, 50u32, Some([0u8, 0u8, 0u8, 0u8]));
	bmp.contents = bytes;

	// Header reads
	let file_header: BITMAPFILEHEADER = BMP::get_header(&bmp);

	let dib_header: DIBHEADER;
	match BMP::get_dib_header(&bmp) {
		Ok(header) => dib_header = header,
		_ => {
			println!("bin_sprite::load_from_bmp() error: Could not read DIB header");
			println!("\tSkipped: {}", &source_file.display());
			return None;
		},
	}

	let width: usize = dib_header.width as usize;
	let height: usize = dib_header.height.abs() as usize;
	let mut bit_depth: u16 = dib_header.bitcount;

	// Cheers Wikipedia
	let row_size: usize = ((bit_depth as usize * width + 31) / 32) * 4;
	let pixel_array_len: usize = row_size * height;

	let start: usize = file_header.bfOffBits as usize;

	let mut pixel_array: Vec<u8> = vec![0; pixel_array_len];
	pixel_array.copy_from_slice(&bmp.contents[start..start + pixel_array_len]);

	// Bit depth handling
	match dib_header.bitcount {
		1 => {
			pixel_array = sprite_transform::bpp_from_1(pixel_array);
			bit_depth = DEPTH_4;
		},

		2 => {
			pixel_array = sprite_transform::bpp_from_2(pixel_array);
			bit_depth = DEPTH_4;
		},

		4 => pixel_array = sprite_transform::bpp_from_4(pixel_array, false),
		8 => (),
		_ => {
			println!("Warning: Skipping BMP as its color depth is not supported ({})", dib_header.bitcount);
			println!("\tSkipped: {}", &source_file.display());
			return None;
		},
	}

	// Trim padding
	let mut pixel_vector = sprite_transform::trim_padding(pixel_array, width, height, true);

	// Invalid BMP
	if std::cmp::max(width, height) > u16::MAX as usize {
		println!("bin_sprite::load_from_bmp() error: image dimensions exceed sprite maximum of 65535px per side");
		println!("\tSkipped: {}", &source_file.display());
		return None;
	}

	if pixel_vector.len() != width * height {
		println!("bin_sprite::load_from_bmp() error: bad BMP: pixel count mismatches image dimensions, result may differ");
		println!("\tFile: {}", &source_file.display());
		pixel_vector.resize(width * height, 0u8);
	}

	let clut: CLUT;
	if with_palette {
		let flags_offset: usize;
		match dib_header.compression {
			Some(value) => match &value as &str {
				"BI_BITFIELDS" => flags_offset = 12,
				"BI_ALPHABITFIELDS" => flags_offset = 16,
				_ => flags_offset = 0,
			},

			None => flags_offset = 0,
		}

		// Bytes per palette color
		let index: usize = 14 + dib_header.size as usize + flags_offset;
		let color_size: usize;
		if dib_header.size as usize == BITMAPCOREHEADER_SIZE {
			color_size = BMP_COLOR_24;
		}
		else {
			color_size = BMP_COLOR_32;
		}

		// How many colors to read from BMP color table
		let color_count: usize;
		match dib_header.ClrUsed {
			Some(value) => match value {
				0 => color_count = 2u16.pow(bit_depth as u32) as usize,
				_ => color_count = value as usize,
			},

			None => color_count = 2u16.pow(bit_depth as u32) as usize,
		}

		// Populate palette
		for color in 0..color_count {
			palette[4 * color + 0] = bmp.contents[index + (color_size * color + 2)];
			palette[4 * color + 1] = bmp.contents[index + (color_size * color + 1)];
			palette[4 * color + 2] = bmp.contents[index + (color_size * color + 0)];

			//
			if color % 32 == 0 || (color as i32 - 8) % 32 == 0 && color != 8 {
				palette[4 * color + 3] = 0x00;
			}
			else {
				palette[4 * color + 3] = 0x80;
			}
		}

		match bit_depth {
			4 => {
				if color_count <= 8
				{ clut = CLUT::Half; }
				else
				{ clut = CLUT::Full; }
			},

			_ => {
				if color_count <= 128
				{ clut = CLUT::Half; }
				else
				{ clut = CLUT::Full; }
			},
		}
	}
	// No palette
	else { clut = CLUT::None }

	return Some(Gd::from_init_fn(|base| {
		BinSprite {
			base,
			mode: Mode::ACPR,
			clut,
			bit_depth,
			width: width as u16,
			height: height as u16,
			texture_width: get_texture_size(width as u16),
			texture_height: get_texture_size(height as u16),
			hash: generate_hash(&pixel_vector),
			manual_hash: false,
			palette,
			pixels: pixel_vector
		}
	}));
}


pub fn load_from_raw(source_file: &PathBuf) -> Option<Gd<BinSprite>> {
	// Find if the RAW file has specified its dimensions
	let mut width: u16 = 0;
	let mut height: u16 = 0;

	let file_name: String = source_file.file_stem().unwrap().to_str().unwrap().to_lowercase();
	let file_name_pieces: Vec<&str> = file_name.split("-").collect();
	let piece_count: usize = file_name_pieces.len();

	for piece in 0..piece_count {
		// Width
		if file_name_pieces[piece] == "w" && piece + 1 < piece_count {
			width = file_name_pieces[piece + 1].parse::<u16>().unwrap_or(0);
		}

		// Height
		if file_name_pieces[piece] == "h" && piece + 1 < piece_count {
			height = file_name_pieces[piece + 1].parse::<u16>().unwrap_or(0);
		}
	}

	if width == 0 {
		println!("Warning: will not process RAW as its width was not specified");
		println!("\tSkipped: {}", &source_file.display());
		return None;
	}

	if height == 0 {
		println!("Warning: will not process RAW as its height was not specified");
		println!("\tSkipped: {}", &source_file.display());
		return None;
	}

	// All good, return raw data
	match fs::read(source_file) {
		Ok(data) => return Some(Gd::from_init_fn(|base| {
			BinSprite {
				base,
				mode: Mode::ACPR,
				clut: CLUT::None,
				bit_depth: DEPTH_8,
				width,
				height,
				texture_width: get_texture_size(width),
				texture_height: get_texture_size(height),
				hash: generate_hash(&data),
				manual_hash: false,
				palette: vec![],
				pixels: data
			}
		})),

		_ => {
			println!("sprite_get::get_raw() error: RAW file read error");
			println!("\tSkipped: {}", &source_file.display());
			return None;
		},
	}
}