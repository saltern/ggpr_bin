use std::fs;
use std::io::Write;
use std::io::BufWriter;
use std::fs::File;
use crate::sprite_transform;
use godot::prelude::*;


#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
/// Rust GGXXAC+R sprite exporter, based on Ghoul.
struct SpriteExporter {}


#[godot_api]
impl SpriteExporter {
	#[func]
	fn make_raw(
		path: String,
		pixels: Vec<u8>,
		width: u16,
		height: u16,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		let mut pixel_vector: Vec<u8> = pixels;

		if sprite_reindex {
			pixel_vector = sprite_transform::reindex_vector(pixel_vector);
		}

		if sprite_flip_h {
			pixel_vector = sprite_transform::flip_h(
				pixel_vector, width as usize, height as usize
			)
		}
		if sprite_flip_v {
			pixel_vector = sprite_transform::flip_v(
				pixel_vector, width as usize, height as usize
			)
		}

		let _ = fs::write(path, pixel_vector);
	}


	#[func]
	fn make_png(
		path: String,
		pixels: Vec<u8>,
		width: u32,
		height: u32,
		bit_depth: u32,
		mut palette: Vec<u8>,
		palette_alpha_mode: u8,
		palette_reindex: bool,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		let png_file: File;
		match File::create(path) {
			Ok(file) => png_file = file,
			_ => return,
		}

		let ref mut buffer = BufWriter::new(png_file);
		let mut encoder = png::Encoder::new(buffer, width, height);

		// 4 bpp handling
		let mut pixel_vector: Vec<u8> = pixels;

		if sprite_flip_h {
			pixel_vector = sprite_transform::flip_h(
				pixel_vector, width as usize, height as usize
			)
		}
		if sprite_flip_v {
			pixel_vector = sprite_transform::flip_v(
				pixel_vector, width as usize, height as usize
			)
		}

		match bit_depth {
			4 => {
				pixel_vector = sprite_transform::align_to_4(pixel_vector, height as usize);
				pixel_vector = sprite_transform::bpp_to_4(pixel_vector, false);
				encoder.set_depth(png::BitDepth::Four);
			},

			_ => {
				if sprite_reindex {
					pixel_vector = sprite_transform::reindex_vector(pixel_vector);
				}

				encoder.set_depth(png::BitDepth::Eight);
			},
		}

		encoder.set_color(png::ColorType::Indexed);

		// Palette
		let color_count: usize = 2usize.pow(bit_depth);

		let mut trns_chunk: Vec<u8> = Vec::new();

		{
			let mut rgb_palette: Vec<u8> = Vec::new();

			if palette_reindex && bit_depth == 8 {
				palette = sprite_transform::reindex_rgba_vector(palette);
			}

			for index in 0..color_count {
				rgb_palette.push(palette[4 * index + 0]);
				rgb_palette.push(palette[4 * index + 1]);
				rgb_palette.push(palette[4 * index + 2]);
				trns_chunk.push(palette[4 * index + 3]);
			}

			encoder.set_palette(rgb_palette);
		}

		if palette_alpha_mode > 0 {
			for index in 0..trns_chunk.len() {
				match palette_alpha_mode {
					// DOUBLE
					1 => {
						if trns_chunk[index] >= 0x80 {
							trns_chunk[index] = 0xFF;
						}

						else {
							trns_chunk[index] *= 2;
						}
					},

					// HALVE
					2 => trns_chunk[index] /= 2,

					// OPAQUE
					_ => trns_chunk[index] = 0xFF,
				}
			}
		}

		encoder.set_trns(trns_chunk);

		let mut writer = encoder.write_header().unwrap();
		writer.write_image_data(&pixel_vector).unwrap();
	}


	fn bmp_header(width: u16, height: u16, bit_depth: u16) -> Vec<u8> {
		let mut bmp_data: Vec<u8> = Vec::new();

		// BITMAPFILEHEADER
		// 2 bytes, "BM"
		bmp_data.push(0x42);
		bmp_data.push(0x4d);

		// 4 bytes, size of the bitmap in bytes
		// 14 bytes - BITMAPFILEHEADER
		// 12 bytes - DIBHEADER of type BITMAPCOREHEADER
		let header_length: u32 = 14 + 12 + 2u32.pow(bit_depth as u32) * 3;
		let bmp_file_size: [u8; 4] = (header_length + (width + width % 4) as u32 * height as u32).to_le_bytes();
		for byte in 0..4 {
			bmp_data.push(bmp_file_size[byte]);
		}

		// 2 bytes each for bfReserved1 and 2
		bmp_data.push(0x00);
		bmp_data.push(0x00);
		bmp_data.push(0x00);
		bmp_data.push(0x00);

		// 4 bytes, offset to pixel array
		bmp_data.push((header_length & 0xFF) as u8);
		bmp_data.push((header_length >> 8) as u8);
		bmp_data.push(0x00);
		bmp_data.push(0x00);

		// DIBHEADER (BITMAPCOREHEADER)
		// 4 bytes, 12
		bmp_data.push(0x0C);
		bmp_data.push(0x00);
		bmp_data.push(0x00);
		bmp_data.push(0x00);

		// 2 bytes, image width
		bmp_data.push(width as u8);
		bmp_data.push((width >> 8) as u8);

		// 2 bytes, image height
		bmp_data.push(height as u8);
		bmp_data.push((height >> 8) as u8);

		// 2 bytes, planes
		bmp_data.push(0x01);
		bmp_data.push(0x00);

		// 2 bytes, bpp
		match bit_depth {
			1 => bmp_data.push(0x01),
			2 => bmp_data.push(0x02),
			4 => bmp_data.push(0x04),
			8 => bmp_data.push(0x08),

			// Theoretically shouldn't happen
			_ => panic!("lib::SpriteExporter::bmp_header() error: Invalid color depth!"),
		}

		bmp_data.push(0x00);
		return bmp_data;
	}


	#[func]
	fn make_bmp(
		path: String,
		pixels: Vec<u8>,
		width: u16,
		height: u16,
		bit_depth: u16,
		mut palette: Vec<u8>,
		palette_reindex: bool,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		// BITMAPFILEHEADER, BITMAPCOREHEADER
		let header: Vec<u8> = Self::bmp_header(width, height, bit_depth);

		// Color table
		let mut color_table: Vec<u8> = Vec::with_capacity(768);
		let color_count: usize = 2usize.pow(bit_depth as u32);

		{
			if palette_reindex && bit_depth == 8 {
				palette = sprite_transform::reindex_rgba_vector(palette);
			}

			for index in 0..color_count {
				color_table.push(palette[4 * index + 2]);
				color_table.push(palette[4 * index + 1]);
				color_table.push(palette[4 * index + 0]);
			}
		}

		// Write out
		let bmp_file: File;
		match File::create(path) {
			Ok(file) => bmp_file = file,
			_ => return Default::default(),
		}

		let mut buffer = BufWriter::new(bmp_file);

		let _ = buffer.write_all(&header);
		let _ = buffer.write_all(&color_table);

		let mut pixel_vector: Vec<u8> = pixels;

		if sprite_flip_h {
			pixel_vector = sprite_transform::flip_h(
				pixel_vector, width as usize, height as usize
			)
		}
		if sprite_flip_v {
			pixel_vector = sprite_transform::flip_v(
				pixel_vector, width as usize, height as usize
			)
		}

		match bit_depth {
			4 => {
				pixel_vector = sprite_transform::align_to_4(pixel_vector, height as usize);
				pixel_vector = sprite_transform::bpp_to_4(pixel_vector, false);
			},

			_ => {
				if sprite_reindex {
					pixel_vector = sprite_transform::reindex_vector(pixel_vector);
				}
			},
		}

		// Cheers Wikipedia
		let row_length: usize = (((bit_depth * width + 31) / 32) * 4) as usize;
		let byte_width: usize = pixel_vector.len() / height as usize;
		let padding: usize = row_length - byte_width;

		// Upside-down write with padding
		for y in (0..height as usize).rev() {
			let row_start: usize = y * byte_width;
			let _ = buffer.write_all(&pixel_vector[row_start..row_start + byte_width]);
			let _ = buffer.write_all(&vec![0u8; padding]);
		}

		let _ = buffer.flush();
	}
}