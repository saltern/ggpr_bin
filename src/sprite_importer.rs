use std::fs;
use std::fs::File;

use crate::sprite_transform;

use bmp_rust::bmp::{BITMAPFILEHEADER, BMP, DIBHEADER};
use color_quant::NeuQuant;
use godot::prelude::*;

const DEPTH_4: u16 = 4;
const DEPTH_8: u16 = 8;

// CLUT values
const CLUT_NONE			: u16 = 0x0000;
const CLUT_HALF			: u16 = 0x0010;
const CLUT_FULL			: u16 = 0x0020;

const CLUT_SIZE_4_HALF	: usize = 4 * COLOR_COUNT_4_HALF;
const CLUT_SIZE_4_FULL	: usize = 4 * COLOR_COUNT_4_FULL;
const CLUT_SIZE_8_HALF	: usize = 4 * COLOR_COUNT_8_HALF;
const CLUT_SIZE_8_FULL	: usize = 4 * COLOR_COUNT_8_FULL;

const COLOR_COUNT_4_HALF: usize = 8;
const COLOR_COUNT_4_FULL: usize = 16;
const COLOR_COUNT_8_HALF: usize = 128;
const COLOR_COUNT_8_FULL: usize = 256;

// Quantization parameters
const QUANT_DEFAULT_QUALITY: i32 = 10;


#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
pub struct ImportData {
	#[export] width: u16,
	#[export] height: u16,
	#[export] bit_depth: u16,
	#[export] clut: u16,
	#[export] palette: PackedByteArray,
	#[export] pixels: PackedByteArray,
}


#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
pub struct SpriteImporter {}


// Palletization
pub fn quantize(rgba: Vec<u8>, bit_depth: u16, quality_level: i32) -> (Vec<u8>, Vec<u8>) {
	//const QUALITY_LEVEL: i32 = 10;

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


#[godot_api]
impl SpriteImporter {
	#[func]
	pub fn load_from_png(source_file: String, with_palette: bool) -> Option<Gd<ImportData>> {
		// Get info
		let file: File;
		match File::open(&source_file) {
			Ok(value) => file = value,
			_ => {
				godot_print!("sprite_import::load_from_png() error: PNG file open error");
				godot_print!("\tSkipped: {source_file}");
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
					godot_print!("Note: PNG has color type grayscale with alpha, will discard alpha");
					godot_print!("\tFile: {source_file}");
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
					godot_print!("Note: PNG has color type RGB, will use red channel as grayscale");
					godot_print!("\tFile: {source_file}");
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
					godot_print!("Note: PNG has color type RGBA, will use red channel as grayscale and discard alpha");
					godot_print!("\tFile: {source_file}");
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

		let clut: u16;

		if palette.is_empty() {
			clut = CLUT_NONE;
		} else {
			match bit_depth {
				DEPTH_4 => {
					if palette.len() <= CLUT_SIZE_4_FULL {
						clut = CLUT_HALF;
						palette.resize(CLUT_SIZE_4_HALF, 0u8);
					} else {
						clut = CLUT_FULL;
						palette.resize(CLUT_SIZE_4_FULL, 0u8);
					}
				},

				_ => {
					if palette.len() <= CLUT_SIZE_8_HALF {
						clut = CLUT_HALF;
						palette.resize(CLUT_SIZE_8_HALF, 0u8);
					}
					else {
						clut = CLUT_FULL;
						palette.resize(CLUT_SIZE_8_FULL, 0u8);
					}
				}
			}
		}

		let width: u16 = reader.info().width as u16;
		let height: u16 = reader.info().height as u16;

		let import_data: Gd<ImportData> = Gd::from_init_fn(|_base| {
			ImportData {
				width,
				height,
				bit_depth,
				clut,
				palette: palette.into(),
				pixels: pixel_vector.into(),
			}
		});

		return Some(import_data);
	}


	#[func]
	pub fn load_from_bmp(source_file: String, with_palette: bool) -> Option<Gd<ImportData>> {
		const BITMAPCOREHEADER_SIZE: usize = 12;
		const BMP_COLOR_24: usize = 3;
		const BMP_COLOR_32: usize = 4;

		// Not using BMP::new_from_file as it does not account for
		// failing to read from a file and will panic if it does
		let mut palette: Vec<u8> = vec![];

		// File read
		let bytes: Vec<u8>;
		match fs::read(&source_file) {
			Ok(value) => bytes = value,
			_ => {
				println!("bin_sprite::load_from_bmp() error: BMP file read error");
				println!("\tSkipped: {source_file}");
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
				println!("\tSkipped: {source_file}");
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
				println!("\tSkipped: {source_file}");
				return None;
			},
		}

		// Trim padding
		let mut pixel_vector = sprite_transform::trim_padding(pixel_array, width, height, true);

		// Invalid BMP
		if std::cmp::max(width, height) > u16::MAX as usize {
			println!("bin_sprite::load_from_bmp() error: image dimensions exceed sprite maximum of 65535px per side");
			println!("\tSkipped: {source_file}");
			return None;
		}

		if pixel_vector.len() != width * height {
			println!("bin_sprite::load_from_bmp() error: bad BMP: pixel count mismatches image dimensions, result may differ");
			println!("\tFile: {source_file}");
			pixel_vector.resize(width * height, 0u8);
		}

		let clut: u16;
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
					{ clut = CLUT_HALF; }
					else
					{ clut = CLUT_FULL; }
				},

				_ => {
					if color_count <= 128
					{ clut = CLUT_HALF; }
					else
					{ clut = CLUT_FULL; }
				},
			}
		}
		// No palette
		else { clut = CLUT_NONE }

		return Some(Gd::from_init_fn(|_base| {
			ImportData {
				width: width as u16,
				height: height as u16,
				bit_depth,
				clut,
				palette: palette.into(),
				pixels: pixel_vector.into(),
			}
		}));
	}


	#[func]
	pub fn load_from_raw(source_file: String) -> Option<Gd<ImportData>> {
		// Find if the RAW file has specified its dimensions
		let mut width: u16 = 0;
		let mut height: u16 = 0;

		let lower = source_file.to_lowercase();
		let file_name_pieces: Vec<&str> = lower.split("-").collect();
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
			godot_print!("Warning: will not process RAW as its width was not specified");
			godot_print!("\tSkipped: {source_file}");
			return None;
		}

		if height == 0 {
			godot_print!("Warning: will not process RAW as its height was not specified");
			godot_print!("\tSkipped: {source_file}");
			return None;
		}

		// All good, return raw data
		match fs::read(&source_file) {
			Ok(data) => {
				return Some(Gd::from_init_fn(|_base| {
					ImportData {
						width,
						height,
						bit_depth: 8,
						palette: vec![].into(),
						clut: CLUT_NONE,
						pixels: data.into(),
					}
				}))
			},

			_ => {
				godot_print!("sprite_get::get_raw() error: RAW file read error");
				godot_print!("\tSkipped: {source_file}");
				return None;
			},
		}
	}
}