use std::cmp::min;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;

use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use bmp_rust::bmp::{BITMAPFILEHEADER, BMP, DIBHEADER};
use color_quant::NeuQuant;

use godot::prelude::*;
use godot::classes::ImageTexture;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::sprite_transform;
use crate::Identification;
use crate::Serialization;
use crate::Deserialization;


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

// Cached image and texture parameters
const IMAGE_EMPTY_W: i32 = 1;
const IMAGE_EMPTY_H: i32 = 1;
const IMAGE_MIPMAPS: bool = false;
const IMAGE_FORMAT: Format = Format::L8;


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
	pixels: Vec<u8>,
	/// Godot auxiliary: cached Image
	#[export] image: Option<Gd<Image>>,
	/// Godot auxiliary: cached ImageTexture
	#[export] texture: Option<Gd<ImageTexture>>,
}


#[godot_api]
impl BinSprite {
	// INIT AND LOAD
	#[func]
	pub fn init_empty_palette() -> Gd<Self> {
		let data = &PackedArray::<u8>::from([0u8]);
		let image = Image::create_from_data(
			IMAGE_EMPTY_W, IMAGE_EMPTY_H, IMAGE_MIPMAPS, IMAGE_FORMAT, data
		);
		let texture = ImageTexture::create_from_image(image.to_godot());

		return Gd::from_init_fn(|base| {
			Self {
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
				image,
				texture,
			}
		})
	}


	#[func]
	pub fn load_from_file(path: String, with_palette: bool) -> Option<Gd<Self>> {
		let source_file: PathBuf = PathBuf::from(path);

		match source_file.extension() {
			Some(os_str) => match os_str.to_ascii_lowercase().to_str() {
				Some("bin") => return Self::load_from_bin(source_file, with_palette),
				Some("png") => return Self::load_from_png(source_file, with_palette),
				Some("bmp") => return Self::load_from_bmp(source_file, with_palette),
				Some("raw") => return Self::load_from_raw(source_file),
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


	pub fn load_from_bin(source_file: PathBuf, with_palette: bool) -> Option<Gd<Self>> {
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
	#[func]
	pub fn has_palette(&self) -> bool {
		match self.clut {
			CLUT::None => return false,
			_ => return !self.palette.is_empty()
		}
	}


	#[func]
	pub fn get_palette(&self) -> Vec<u8> {
		// Guard rail
		match self.clut {
			CLUT::None => Vec::new(),
			_ => self.palette.clone(),
		}
	}


	#[func]
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


	#[func]
	pub fn get_color(&self, index: i64) -> Color {
		let index = index as usize;

		if index >= self.get_color_count() {
			return Color::from_rgba8(0, 0, 0, 0);
		} else {
			return Color::from_rgba8(
				self.palette[4 * index + 0],
				self.palette[4 * index + 1],
				self.palette[4 * index + 2],
				self.palette[4 * index + 3],
			);
		}
	}


	#[func]
	pub fn set_color(&mut self, index: i64, r: u8, g: u8, b: u8, a: u8) {
		let index = index as usize;

		if index >= self.get_color_count() {
			return;
		}

		self.palette[4 * index + 0] = r;
		self.palette[4 * index + 1] = g;
		self.palette[4 * index + 2] = b;
		self.palette[4 * index + 3] = a;
	}


	pub fn purge_palette(&mut self) {
		self.palette = vec![];
		self.clut = CLUT::None;
	}


	pub fn palette_halve_alpha(&mut self) {
		for index in 0..self.get_color_count() {
			let a: usize = 4 * index + 3;

			if self.palette[a] == 0xFF {
				self.palette[a] = 0x80;
			} else {
				self.palette[a] /= 2;
			}
		}
	}


	pub fn palette_double_alpha(&mut self) {
		for index in 0..self.get_color_count() {
			let a: usize = 4 * index + 3;

			if self.palette[a] >= 0x80 {
				self.palette[a] = 0xFF;
			} else {
				self.palette[a] *= 2;
			}
		}
	}


	pub fn palette_make_opaque(&mut self) {
		for index in 0..self.get_color_count() {
			let a: usize = 4 * index + 3;
			self.palette[a] = 0xFF;
		}
	}


	// BIT DEPTH
	#[func]
	pub fn get_bit_depth(&self) -> u16 {
		if self.bit_depth == DEPTH_4
		{ return DEPTH_4; }
		else
		{ return DEPTH_8; }
	}


	#[func]
	pub fn set_bit_depth_4(&mut self) {
		self.bit_depth = DEPTH_4;
	}


	#[func]
	pub fn set_bit_depth_8(&mut self) {
		self.bit_depth = DEPTH_8;
	}


	// DIMENSIONS
	#[func]
	pub fn get_width(&self) -> u16 {
		return self.width;
	}


	#[func]
	pub fn get_height(&self) -> u16 {
		return self.height;
	}


	pub fn get_texture_width(&self) -> u16 {
		return 2u16.pow(self.texture_width as u32);
	}


	pub fn get_texture_height(&self) -> u16 {
		return 2u16.pow(self.texture_height as u32);
	}


	// PIXEL VECTOR
	#[func]
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
				image: self.image.clone(),
				texture: self.texture.clone(),
			}
		});
	}

/*
	#[func]
	pub fn get_image(&self) -> Option<Gd<Image>> {
		return self.image.clone();
	}


	#[func]
	pub fn get_texture(&self) -> Option<Gd<ImageTexture>> {
		return self.texture.clone();
	}
 */
}


#[godot_api]
impl IResource for BinSprite {
	fn init(base: Base<Resource>) -> Self {
		let data = &PackedArray::<u8>::from([0u8]);
		let image = Image::create_from_data(IMAGE_EMPTY_W, IMAGE_EMPTY_H, IMAGE_MIPMAPS, IMAGE_FORMAT, data);
		let texture = ImageTexture::create_from_image(image.to_godot());

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
			image,
			texture,
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
		let pixels: Vec<u8>;

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
			Mode::Mode5 => pixels = decompress_mode5(&pixel_data),
			Mode::GGXP => {
				if ggxp_compressed { pixels = decompress_ggx(bin_data); }
				else { pixels = pixel_data; }
			},
		}

		// Finishing touches
		//pixels = sprite_transform::trim_padding(pixels, width as usize, height as usize, false);

		let data = &PackedArray::<u8>::from(pixels.clone());
		let image = Image::create_from_data(
			width as i32, height as i32, IMAGE_MIPMAPS, IMAGE_FORMAT, data
		);
		let texture = ImageTexture::create_from_image(image.to_godot());

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
				image,
				texture,
			}
		}));
	}
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