use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::ops::Deref;

use godot::prelude::*;
use godot::classes::IResource;
use godot::classes::Image;
use godot::classes::image::Format;
use godot::classes::ImageTexture;

pub mod bin_sprite;
pub mod bin_palette;
pub mod decompressor;
pub mod sprite_transform;
pub mod sort;

use crate::{
	bin_sprite::{BinHeader, BinSprite},
	decompressor::{SpriteData},
};

struct GodotGhoul;


#[gdextension]
unsafe impl ExtensionLibrary for GodotGhoul {}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite decompressor and loader, based on Ghoul.
struct SpriteLoader {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for SpriteLoader {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl SpriteLoader {
	/// Loads BIN sprites in natural naming order from a specified path.
	#[func]
	fn load_sprites(g_path: GString, reindex: bool) -> Array<Gd<BinSprite>> {
		let path_str: String = String::from(g_path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find sprite directory!");
			return Default::default();
		}
		
		let mut file_vector: Vec<PathBuf> = Vec::new();
		
		match fs::read_dir(path_buf) {
			Ok(value) => {
				for entry in value {
					file_vector.push(entry.unwrap().path());
				}
			},
			
			_ => return Default::default(),
		}
		
		file_vector.sort_by(|a, b| natord::compare(a.to_str().unwrap(), b.to_str().unwrap()));
		let mut sprite_vector: Array<Gd<BinSprite>> = array![];
		
		for item in file_vector {
			match item.extension() {
				Some(value) => {
					if value != "bin" {
						continue
					}
				},
				
				_ => continue,
			}
			
			let bin_data: Vec<u8>;
			
			match fs::read(item) {
				Ok(data) => bin_data = data,
				_ => continue,
			}
			
			let bin_header: BinHeader = bin_sprite::get_header(bin_data.clone());
			let sprite_data: SpriteData;
			
			if bin_header.compressed {
				sprite_data = decompressor::decompress(bin_data, bin_header, reindex);
			}
		
			// Handle uncompressed
			else {
				let mut pointer: usize = 0x10;
				let palette: Vec<u8>;
				
				// Have embedded palette
				if bin_header.clut == 0x20 {
					let color_count: usize = 2usize.pow(bin_header.bit_depth as u32);
					palette = bin_data[0x10..0x10 + color_count * 4].to_vec();
					pointer = 0x10 + color_count * 4;
				}
				
				else {
					palette = Vec::new();
				}
				
				// Get pixels
				let mut byte_vector: Vec<u8> = bin_data[pointer..].to_vec();
				
				if bin_header.bit_depth == 8 && reindex {
					byte_vector = sprite_transform::reindex_vector(byte_vector);
				}
				else if bin_header.bit_depth == 4 {
					byte_vector = sprite_transform::bpp_from_4(byte_vector);
				}
				
				// Truncate
				byte_vector.resize((bin_header.width as usize) * (bin_header.height as usize), 0u8);
				
				sprite_data = SpriteData {
					width: bin_header.width,
					height: bin_header.height,
					bit_depth: bin_header.bit_depth,
					pixels: byte_vector,
					palette: palette,
				}
			}
			
			let image: Gd<Image>;
			
			match Image::create_from_data(
				// Width
				sprite_data.width as i32,
				// Height
				sprite_data.height as i32,
				// Mipmapping
				false,
				// Grayscale format
				Format::L8,
				// Pixel array
				PackedByteArray::from(sprite_data.pixels.clone())
			) {
				Some(gd_image) => image = gd_image,
				_ => continue,
			}
			
			sprite_vector.push(
				BinSprite::new_from_data(
					// Pixels
					PackedByteArray::from(sprite_data.pixels),
					// Image
					image,
					// Color depth
					sprite_data.bit_depth,
					// Palette
					PackedByteArray::from(sprite_data.palette)
				)
			);
		}
		
		return sprite_vector;
	}
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite exporter, based on Ghoul.
struct SpriteExporter {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for SpriteExporter {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl SpriteExporter {
	fn make_bin(
		file_path: PathBuf,
		sprite: &BinSprite,
		palette_include: bool,
		external_palette: Vec<u8>,
		palette_alpha_mode: u64,
		palette_override: bool,
		reindex: bool
	) {
		let clut: u16;
		let mut palette: Vec<u8>;
		
		// No palette
		if !palette_include {
			clut = 0x00;
			palette = Vec::new();
		}
		
		// Palette included
		else {
			clut = 0x20;
			
			// Sprite has no embedded palette or is forced override
			if sprite.palette.is_empty() || palette_override {
				palette = external_palette;
			}
			
			// Sprite has embedded palette
			else {
				palette = sprite.palette.to_vec();
			}
		}
		
		for index in 0..palette.len() / 4 {
			match palette_alpha_mode {
				// AS_IS
				0 => (),
				
				// DOUBLE
				1 => {
					if palette[4 * index + 3] >= 0x80 {
						palette[4 * index + 3] = 0xFF;
					}
					
					else {
						palette[4 * index + 3] = palette[4 * index + 3] * 2;
					}
				},
				
				// HALVE
				2 => palette[4 * index + 3] = palette[4 * index + 3] / 2,
				
				// OPAQUE
				_ => palette[4 * index + 3] = 0xFF,
			}
		}
		
		let pixel_vector: Vec<u8>;
		if reindex {
			pixel_vector = sprite_transform::reindex_vector(sprite.pixels.to_vec());
		}
		
		else {
			pixel_vector = sprite.pixels.to_vec();
		}
		
		let image: Gd<Image> = sprite.image.clone().unwrap();
		let width = image.get_width() as u16;
		let height = image.get_height() as u16;
		
		let header_bytes: Vec<u8> = bin_sprite::make_header(
			false, clut, sprite.bit_depth, width, height, 0, 0, 0);
		
		let bin_file: File;
		match File::create(&file_path) {
			Ok(file) => bin_file = file,
			_ => return Default::default(),
		}
		
		let mut buffer = BufWriter::new(bin_file);
		let _ = buffer.write_all(&header_bytes);
		
		if clut == 0x20 {
			let _ = buffer.write_all(&palette);
		}
		
		let _ = buffer.write_all(&pixel_vector);
		let _ = buffer.flush();
	}
	

	fn make_raw(
		mut path_buf: PathBuf,
		name_index: u64,
		sprite: &BinSprite,
		reindex: bool
	) {
		let image: Gd<Image> = sprite.image.clone().unwrap();
		let width = image.get_width();
		let height = image.get_height();
		
		path_buf.push(format!("sprite_{}-W-{}-H-{}.raw", name_index, width, height));
		
		let pixel_vector: Vec<u8>;
		if reindex {
			pixel_vector = sprite_transform::reindex_vector(sprite.pixels.to_vec());
		}
		
		else {
			pixel_vector = sprite.pixels.to_vec();
		}
		
		let raw_file: File;
		match File::create(&path_buf) {
			Ok(file) => raw_file = file,
			_ => return Default::default(),
		}
		
		let mut buffer = BufWriter::new(raw_file);
		let _ = buffer.write_all(&pixel_vector);
		let _ = buffer.flush();
	}
	
	
	fn make_png(
		file_path: PathBuf,
		sprite: &BinSprite,
		palette_include: bool,
		external_palette: Vec<u8>,
		palette_alpha_mode: u64,
		palette_override: bool,
		reindex: bool
	) {
		let png_file: File;
		match File::create(&file_path) {
			Ok(file) => png_file = file,
			_ => return,
		}
		
		let image: Gd<Image> = sprite.image.clone().unwrap();
		
		let width: u32 = image.get_width() as u32;
		let height: u32 = image.get_height() as u32;
		
		let ref mut buffer = BufWriter::new(png_file);
		let mut encoder = png::Encoder::new(buffer, width, height);
		
		// 4 bpp handling
		let working_pixels: Vec<u8>;
		
		match sprite.bit_depth {
			4 => {
				working_pixels = sprite_transform::bpp_to_4(sprite.pixels.to_vec(), false);
				encoder.set_depth(png::BitDepth::Four);
			},
			
			8 => {
				if reindex {
					working_pixels = sprite_transform::reindex_vector(sprite.pixels.to_vec());
				}
				
				else {
					working_pixels = sprite.pixels.to_vec();
				}
				
				encoder.set_depth(png::BitDepth::Eight);
			},
			
			_ => return,
		}	
		
		encoder.set_color(png::ColorType::Indexed);
		
		// Palette
		let color_count: usize = 2usize.pow(sprite.bit_depth as u32);
		
		let mut trns_chunk: Vec<u8> = Vec::new();
		
		if sprite.palette.is_empty() || palette_override || !palette_include {
			let mut rgb_palette: Vec<u8> = Vec::new();
			
			for index in 0..color_count {
				rgb_palette.push(external_palette[4 * index + 0]);
				rgb_palette.push(external_palette[4 * index + 1]);
				rgb_palette.push(external_palette[4 * index + 2]);
				trns_chunk.push(external_palette[4 * index + 3]);
			}
			
			encoder.set_palette(rgb_palette);
		}
		
		else {
			let pal_vec: Vec<u8> = sprite.palette.to_vec();
			let mut rgb_palette: Vec<u8> = Vec::new();
			
			for index in 0..color_count {
				rgb_palette.push(pal_vec[4 * index + 0]);
				rgb_palette.push(pal_vec[4 * index + 1]);
				rgb_palette.push(pal_vec[4 * index + 2]);
				trns_chunk.push(pal_vec[4 * index + 3]);
			}
			
			encoder.set_palette(rgb_palette);
		}
		
		for index in 0..trns_chunk.len() {
			match palette_alpha_mode {
				// AS_IS
				0 => (),
				
				// DOUBLE
				1 => {
					if trns_chunk[index] >= 0x80 {
						trns_chunk[index] = 0xFF;
					}
					
					else {
						trns_chunk[index] = trns_chunk[index] * 2;
					}
				},
				
				// HALVE
				2 => trns_chunk[index] = trns_chunk[index] / 2,
				
				// OPAQUE
				_ => trns_chunk[index] = 0xFF,
			}
		}
		
		encoder.set_trns(trns_chunk);
		
		let mut writer = encoder.write_header().unwrap();
		writer.write_image_data(&working_pixels).unwrap();
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
	
	
	fn make_bmp(
		file_path: PathBuf,
		sprite: &BinSprite,
		palette_include: bool,
		external_palette: Vec<u8>,
		palette_override: bool,
		reindex: bool
	) {
		let image: Gd<Image> = sprite.image.clone().unwrap();
		let width: u16 = image.get_width() as u16;
		let height: u16 = image.get_height() as u16;
		
		// BITMAPFILEHEADER, BITMAPCOREHEADER
		let header: Vec<u8> = Self::bmp_header(width, height, sprite.bit_depth);
		
		// Color table
		let mut color_table: Vec<u8> = Vec::with_capacity(768);
		let color_count: usize = 2usize.pow(sprite.bit_depth as u32);
		
		// if palette.is_empty() || palette_override: external_palette
		// else sprite_palette
		
		// Grayscale
		if sprite.palette.is_empty() || palette_override || !palette_include {
			for index in 0..color_count {
				color_table.push(external_palette[4 * index + 2]);
				color_table.push(external_palette[4 * index + 1]);
				color_table.push(external_palette[4 * index + 0]);
			}
		}
		
		// Palette (no alpha)
		else {
			for index in 0..color_count {
				color_table.push(sprite.palette[4 * index + 2]);
				color_table.push(sprite.palette[4 * index + 1]);
				color_table.push(sprite.palette[4 * index + 0]);
			}
		}
		
		// Write out
		let bmp_file: File;
		match File::create(&file_path) {
			Ok(file) => bmp_file = file,
			_ => return Default::default(),
		}
			
		let mut buffer = BufWriter::new(bmp_file);
		
		let _ = buffer.write_all(&header);
		let _ = buffer.write_all(&color_table);
		
		let byte_vector: Vec<u8>;
		
		match sprite.bit_depth {
			// 1 and 2 bpp not currently in use
			4 => byte_vector = sprite_transform::bpp_to_4(sprite.pixels.to_vec(), false),
			8 => {
				if reindex {
					byte_vector = sprite_transform::reindex_vector(sprite.pixels.to_vec());
				}
				
				else {
					byte_vector = sprite.pixels.to_vec();
				}
			},
			
			// Shouldn't happen
			_ => panic!("sprite_make::make_bmp() error: Invalid bit depth"),
		}
		
		// Cheers Wikipedia
		let row_length: usize = (((sprite.bit_depth * width + 31) / 32) * 4) as usize;
		let byte_width: usize = byte_vector.len() / height as usize;
		let padding: usize = row_length - byte_width;
		
		// Upside-down write with padding
		for y in (0..height as usize).rev() {
			let row_start: usize = y * byte_width;
			let _ = buffer.write_all(&byte_vector[row_start..row_start + byte_width]);
			let _ = buffer.write_all(&vec![0u8; padding]);
		}
		
		let _ = buffer.flush();
	}
	
	
	/// Saves sprites in the specified format at the specified path.
	#[func]
	fn export_sprites(
		g_format: GString,
		g_path: GString,
		g_sprites: Vec<Gd<BinSprite>>,
		name_start_index: u64,
		palette_include: bool,
		g_palette: PackedByteArray,
		palette_alpha_mode: u64,
		palette_override: bool,
		reindex: bool
	) {
		let path_str: String = String::from(g_path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find export directory!");
			return Default::default();
		}
		
		let mut name_index: u64 = name_start_index;
		
		// Loop over sprites
		for mut sprite in g_sprites {
			let mut file_path: PathBuf = path_buf.clone();
			
			match g_format.to_string().as_str() {
				"bin" => {
					file_path.push(format!("sprite_{}.bin", name_index));
					Self::make_bin(
						file_path,
						sprite.bind_mut().deref(),
						palette_include,
						g_palette.to_vec(),
						palette_alpha_mode,
						palette_override,
						reindex
					);
				},
			
				"png" => {
					file_path.push(format!("sprite_{}.png", name_index));
					Self::make_png(
						file_path,
						sprite.bind_mut().deref(),
						palette_include,
						g_palette.to_vec(),
						palette_alpha_mode,
						palette_override,
						reindex
					);
				},
				
				"bmp" => {
					file_path.push(format!("sprite_{}.bmp", name_index));
					Self::make_bmp(
						file_path,
						sprite.bind_mut().deref(),
						palette_include,
						g_palette.to_vec(),
						palette_override,
						reindex
					);
				},
				
				"raw" => {
					Self::make_raw(
						file_path,
						name_index,
						sprite.bind_mut().deref(),
						reindex
					);
				},
					
				
				&_ => (),
			}
			
			name_index += 1;
		}
	}
}