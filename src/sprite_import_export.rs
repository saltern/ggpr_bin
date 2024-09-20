use std::io::Write;
use std::io::BufWriter;
use std::fs::File;
use std::path::PathBuf;
use std::ops::Deref;

use godot::prelude::*;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::bin_sprite;
use crate::sprite_get;
use crate::sprite_compress;
use crate::sprite_transform;

use bin_sprite::BinSprite;
use sprite_compress::SpriteData;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite importer, based on Ghoul.
struct SpriteImporter {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for SpriteImporter{
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl SpriteImporter {
	/// Signals import progress.
	#[signal]
	fn sprite_imported();
	
	/// Imports the given sprites.
	#[func]
	fn import_sprites(
		&mut self,
		sprites: PackedStringArray,
		embed_palette: bool,
		halve_alpha: bool,
		flip_h: bool,
		flip_v: bool,
		as_rgb: bool,
		reindex: bool,
		bit_depth: i64,
	) -> Array<Gd<BinSprite>> {
		let file_vector: Vec<GString> = sprites.to_vec();
		let mut sprite_vector: Array<Gd<BinSprite>> = array![];
		
		for item in file_vector {
			match Self::import_sprite(
				item, embed_palette, halve_alpha, flip_h, flip_v, as_rgb, reindex, bit_depth
			) {
				Some(bin_sprite) => sprite_vector.push(bin_sprite),
				None => continue,
			}
			
			// This is dumb as hell
			self.base_mut().call_deferred("emit_signal".into(), &["sprite_imported".to_variant()]);
		}
		
		return sprite_vector;
	}
	
	#[func]
	fn import_sprite(
		file_path: GString,
		embed_palette: bool,
		halve_alpha: bool,
		flip_h: bool,
		flip_v: bool,
		as_rgb: bool,
		reindex: bool,
		bit_depth: i64,
	) -> Option<Gd<BinSprite>> {
		let file_string: String = String::from(file_path);
		let file: PathBuf = PathBuf::from(file_string);
		
		if !file.exists() {
			return None;
		}
		
		let mut data: SpriteData = SpriteData::default();
		
		match file.extension() {
			Some(os_str) => match os_str.to_ascii_lowercase().to_str() {
				Some("png") => data = sprite_get::get_png(&file),
				Some("raw") => data = sprite_get::get_raw(&file),
				Some("bin") => data = sprite_get::get_bin(&file),
				Some("bmp") => data = sprite_get::get_bmp(&file),
				_ => godot_print!("lib::import_sprites() error: Invalid source format provided"),
			},
			
			_ => godot_print!("lib::import_sprites() error: Invalid source format provided"),
		}
		
		if data.width == 0 || data.height == 0 {
			godot_print!("Skipping file as it is empty");
			godot_print!("\tFile: {:?}", file);
			return None;
		}
		
		// As RGB (needs to happen before palette embed)
		if as_rgb && !data.palette.is_empty() {
			data.pixels = sprite_transform::indexed_as_rgb(data.pixels, &data.palette);
		}
		
		// Embed palette
		if embed_palette && !data.palette.is_empty() {
			let mut temp_palette: Vec<u8> = data.palette;
			let color_count: usize = 2usize.pow(data.bit_depth as u32);
			
			// Expand palette
			if temp_palette.len() < 4 * color_count {
				for index in 0..color_count - (temp_palette.len() / 4) {
					// RGB
					temp_palette.push(0x00);
					temp_palette.push(0x00);
					temp_palette.push(0x00);
					
					// Default alpha
					if (index / 16) % 2 == 0 && index % 8 == 0 && index != 8 {
						temp_palette.push(0x00);
					} else {
						temp_palette.push(0x80);
					}
				}
			}
			
			// Truncate palette
			else {
				temp_palette.resize(color_count * 4, 0u8);
			}
			
			data.palette = temp_palette;
		}
		
		// Don't embed palette
		else {
			data.palette = Vec::new();
		}
		
		// Halve alpha
		if halve_alpha {
			data.palette = sprite_transform::alpha_halve(data.palette);
		}
		
		// Flip H/V
		if flip_h {
			data.pixels = sprite_transform::flip_h(data.pixels, data.width as usize, data.height as usize);
		}
		
		if flip_v {
			data.pixels = sprite_transform::flip_v(data.pixels, data.width as usize, data.height as usize);
		}
		
		// Reindex
		if reindex {
			data.pixels = sprite_transform::reindex_vector(data.pixels);
		}
		
		// Forced bit depth
		match bit_depth {
			1 => data.bit_depth = 4,
			2 => data.bit_depth = 8,
			_ => data.bit_depth = std::cmp::max(data.bit_depth, 4),
		}
		
		// Now, create BinSprite.
		let image: Gd<Image>;
		
		match Image::create_from_data(
			// Width
			data.width as i32,
			// Height
			data.height as i32,
			// Mipmapping
			false,
			// Grayscale format
			Format::L8,
			// Pixel array
			PackedByteArray::from(data.pixels.clone())
		) {
			Some(gd_image) => image = gd_image,
			_ => return None,
		}
		
		return Some(BinSprite::new_from_data(
			// Pixels
			PackedByteArray::from(data.pixels),
			// Image
			image,
			// Color depth
			data.bit_depth,
			// Palette
			PackedByteArray::from(data.palette)
		));
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