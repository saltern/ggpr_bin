use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::ops::Deref;

use godot::prelude::*;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::{bin_sprite, Serialization};
//use crate::bin_palette::BinPalette;
//use crate::sprite_get;
//use crate::sprite_compress;
use crate::sprite_transform;

use bin_sprite::BinSprite;
//use sprite_compress::SpriteData;

use color_quant::NeuQuant;

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
			base,
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
		reindex_sprite: bool,
		reindex_palette: bool,
		bit_depth: i64,
		//generate_palette: bool,
		quality_level: i32,
	) -> Array<Gd<BinSprite>> {
		let file_vector: Vec<GString> = sprites.to_vec();
		let mut sprite_vector: Array<Gd<BinSprite>> = array![];
		
		for item in file_vector {
			match Self::import_sprite(
				item, embed_palette, halve_alpha, flip_h, flip_v, as_rgb,
				reindex_sprite, reindex_palette, bit_depth,
				/*generate_palette,*/ quality_level
			) {
				Some(bin_sprite) => sprite_vector.push(&bin_sprite),
				None => continue,
			}
			
			// This is dumb as hell
			self.base_mut().call_deferred("emit_signal", &["sprite_imported".to_variant()]);
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
		reindex_sprite: bool,
		reindex_palette: bool,
		bit_depth: i64,
		quality_level: i32,
	) -> Option<Gd<BinSprite>> {
		let file_string: String = String::from(file_path);
		let file: PathBuf = PathBuf::from(file_string);
		
		if !file.exists() {
			return None;
		}
		
		let mut sprite: Gd<BinSprite>;
		
		match bin_sprite::load_from_file(&file, embed_palette) {
			Some(binsprite) => sprite = binsprite,
			None => return None,
		}

		{
			let mut sprite_bind = sprite.bind_mut();

			// As RGB (needs to happen before palette embed)
			if as_rgb && sprite_bind.has_palette() {
				let pixels: Vec<u8> = sprite_bind.get_pixels();
				let palette: Vec<u8> = sprite_bind.get_palette();
				sprite_bind.set_pixels(sprite_transform::indexed_as_rgb(pixels, palette));
			}

			// Forced bit depth
			match bit_depth {
				1 => sprite_bind.set_bit_depth_4(),
				2 => sprite_bind.set_bit_depth_8(),
				_ => () //data.bit_depth = std::cmp::max(data.bit_depth, 4),
			}

			if sprite_bind.get_bit_depth() == 4 {
				let pixels = sprite_bind.get_pixels();
				sprite_bind.set_pixels(sprite_transform::limit_16_colors(pixels));
			}

			// Halve alpha
			if halve_alpha {
				sprite_bind.palette_halve_alpha();
			}

			// Flip H/V
			if flip_h { sprite_bind.flip_h(); }
			if flip_v { sprite_bind.flip_v(); }

			// Reindex
			if reindex_sprite { sprite_bind.reindex_pixels(); }
			if reindex_palette { sprite_bind.reindex_palette(); }
		}

		return Some(sprite);
	}
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Object containing SpriteExporter settings
struct SpriteExporterSettings {
	base: Base<Resource>,
	/// Whether export to compressed .bin should be performed.
	#[export] export_bin: bool,
	/// Whether export to uncompressed .bin should be performed.
	#[export] export_bin_uncompressed: bool,
	/// Whether export to .raw should be performed.
	#[export] export_raw: bool,
	/// Whether export to indexed .png should be performed.
	#[export] export_png: bool,
	/// Whether export to indexed .bmp should be performed.
	#[export] export_bmp: bool,
	/// Whether the export should include the global or embedded palette.
	/// If set to false, a black-to-white gradient palette will be used
	/// for PNG and BMP exports.
	#[export] palette_include: bool,
	/// Alpha processing to apply to palette.
	/// 0: Leave alpha as-is
	/// 1: Double all alpha values
	/// 2: Halve all alpha values
	/// 3: Make every color fully opaque
	#[export] palette_alpha_mode: u8,
	/// The actual palette colors. R, G, B, A.
	#[export] palette_colors: PackedByteArray,
	/// Whether the palette override option is ticked.
	/// Forces the use of the external palette (`palette_colors`).
	#[export] palette_override: bool,
	/// Whether the palette should be reindexed.
	/// Can be used to not need to reindex again on import.
	#[export] palette_reindex: bool,
	/// Whether the sprite should be reindexed.
	#[export] sprite_reindex: bool,
	/// Mirrors the sprite horizontally.
	#[export] sprite_flip_h: bool,
	/// Mirrors the sprite vertically.
	#[export] sprite_flip_v: bool,
	/// Starting sprite number to use for file names.
	#[export] file_name_start_index: u32,
	/// Pads the sprite number with leading zeroes if necessary,
	/// e.g. if exporting 11 to 100 sprites, one zero will be added;
	/// if exporting 101 to 1000 sprites, two zeroes will be added. 
	#[export] file_name_zero_pad: bool,
}


#[godot_api]
impl IResource for SpriteExporterSettings {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			export_bin: false,
			export_bin_uncompressed: false,
			export_raw: false,
			export_png: false,
			export_bmp: false,
			palette_include: false,
			palette_alpha_mode: 0u8,
			palette_colors: PackedByteArray::from([]),
			palette_override: false,
			palette_reindex: false,
			sprite_reindex: false,
			sprite_flip_h: false,
			sprite_flip_v: false,
			file_name_start_index: 0u32,
			file_name_zero_pad: false,
		}
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
			base,
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
		palette_alpha_mode: u8,
		palette_override: bool,
		palette_reindex: bool,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
		compress: bool,
	) {
		let mut n_sprite: Gd<BinSprite> = sprite.clone();
		let mut clone = n_sprite.bind_mut();

		// No palette
		if !palette_include {
			clone.purge_palette();
		}
		
		// Palette included
		else {
			// Sprite has no embedded palette or is forced override
			if !clone.has_palette() || palette_override {
				clone.set_palette(external_palette);
			}
		}

		match palette_alpha_mode {
			0 => (),
			1 => clone.palette_double_alpha(),
			2 => clone.palette_halve_alpha(),
			_ => clone.palette_make_opaque(),
		}

		// Reindex 8-bit sprites only
		if palette_reindex {
			clone.reindex_palette();
		}
		
		// Reindex 8-bit sprites only
		if sprite_reindex {
			clone.reindex_pixels();
		}
		
		if sprite_flip_h { clone.flip_h(); }
		if sprite_flip_v { clone.flip_v(); }

		if compress {
			clone.set_mode(1);
		} else {
			clone.set_mode(0);
		}

		let bin_file: File;
		match File::create(&file_path) {
			Ok(file) => bin_file = file,
			_ => return Default::default(),
		}

		let mut buffer = BufWriter::new(bin_file);
		let _ = buffer.write_all(&clone.serialize());
		let _ = buffer.flush();
	}
	

	fn make_raw(
		mut path_buf: PathBuf,
		name_index: String,
		sprite: &BinSprite,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		let width = sprite.get_width();
		let height = sprite.get_height();
		
		path_buf.push(format!("sprite_{}-W-{}-H-{}.raw", name_index, width, height));
		
		let mut pixel_vector: Vec<u8> = sprite.get_pixels();
		
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
		palette_alpha_mode: u8,
		palette_override: bool,
		palette_reindex: bool,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		let png_file: File;
		match File::create(&file_path) {
			Ok(file) => png_file = file,
			_ => return,
		}
		
		let width: u32 = sprite.get_width() as u32;
		let height: u32 = sprite.get_height() as u32;
		
		let ref mut buffer = BufWriter::new(png_file);
		let mut encoder = png::Encoder::new(buffer, width, height);
		
		// 4 bpp handling
		let mut pixel_vector: Vec<u8> = sprite.get_pixels();

		if sprite_flip_h {
			pixel_vector = sprite_transform::flip_h(
				pixel_vector, sprite.get_width() as usize, sprite.get_height() as usize
			)
		}
		if sprite_flip_v {
			pixel_vector = sprite_transform::flip_v(
				pixel_vector, sprite.get_width() as usize, sprite.get_height() as usize
			)
		}
		
		match sprite.get_bit_depth() {
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
		let color_count: usize = 2usize.pow(sprite.get_bit_depth() as u32);
		
		let mut trns_chunk: Vec<u8> = Vec::new();

		{
			let mut pal_vec: Vec<u8>;
			let mut rgb_palette: Vec<u8> = Vec::new();
			
			if !sprite.has_palette() || palette_override || !palette_include {
				pal_vec = external_palette;
			} else {
				pal_vec = sprite.get_palette();
			}
			
			if palette_reindex && sprite.get_bit_depth() == 8 {
				pal_vec = sprite_transform::reindex_rgba_vector(pal_vec);
			}
			
			for index in 0..color_count {
				rgb_palette.push(pal_vec[4 * index + 0]);
				rgb_palette.push(pal_vec[4 * index + 1]);
				rgb_palette.push(pal_vec[4 * index + 2]);
				trns_chunk.push(pal_vec[4 * index + 3]);
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
	
	
	fn make_bmp(
		file_path: PathBuf,
		sprite: &BinSprite,
		palette_include: bool,
		external_palette: Vec<u8>,
		palette_override: bool,
		palette_reindex: bool,
		sprite_reindex: bool,
		sprite_flip_h: bool,
		sprite_flip_v: bool,
	) {
		//let image: Gd<Image> = sprite.image.clone().unwrap();
		let width: u16 = sprite.get_width() as u16;
		let height: u16 = sprite.get_height() as u16;
		
		// BITMAPFILEHEADER, BITMAPCOREHEADER
		let header: Vec<u8> = Self::bmp_header(width, height, sprite.get_bit_depth());
		
		// Color table
		let mut color_table: Vec<u8> = Vec::with_capacity(768);
		let color_count: usize = 2usize.pow(sprite.get_bit_depth() as u32);

		{
			let mut pal_vec: Vec<u8>;
			
			if !sprite.has_palette() || palette_override || !palette_include {
				// Grayscale
				pal_vec = external_palette;
			} else {
				// Palette (no alpha)
				pal_vec = sprite.get_palette();
			}
			
			if palette_reindex && sprite.get_bit_depth() == 8 {
				pal_vec = sprite_transform::reindex_rgba_vector(pal_vec);
			}
			
			for index in 0..color_count {
				color_table.push(pal_vec[4 * index + 2]);
				color_table.push(pal_vec[4 * index + 1]);
				color_table.push(pal_vec[4 * index + 0]);
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
		
		let mut pixel_vector: Vec<u8> = sprite.get_pixels();

		if sprite_flip_h {
			pixel_vector = sprite_transform::flip_h(
				pixel_vector, sprite.get_width() as usize, sprite.get_height() as usize
			)
		}
		if sprite_flip_v {
			pixel_vector = sprite_transform::flip_v(
				pixel_vector, sprite.get_width() as usize, sprite.get_height() as usize
			)
		}
		
		match sprite.get_bit_depth() {
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
		let row_length: usize = (((sprite.get_bit_depth() * width + 31) / 32) * 4) as usize;
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
	
	
	/// Saves sprites in the specified format at the specified path.
	#[func]
	fn export_sprites(
		sprites: Vec<Gd<BinSprite>>,
		output_path: GString,
		settings: Gd<SpriteExporterSettings>,
	) {
		let settings = settings.bind();
		let path_str: String = String::from(output_path.clone());
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find export directory!");
			return Default::default();
		}

		let mut name_index: u32 = settings.file_name_start_index;

		let padding: usize;
		if settings.file_name_zero_pad {
			let max_sprite: usize = settings.file_name_start_index as usize + sprites.len() - 1;
			padding = (max_sprite.checked_ilog10().unwrap_or(0) + 1) as usize;
		} else {
			padding = 0;
		}
		
		// Loop over sprites
		for mut sprite in sprites {
			if settings.export_bin {
				let mut file_path: PathBuf = path_buf.clone();
				file_path.push(format!("sprite_{:0padding$}.bin", name_index));

				Self::make_bin(
					file_path,
					&sprite.bind_mut(),
					settings.palette_include,
					settings.palette_colors.to_vec(),
					settings.palette_alpha_mode,
					settings.palette_override,
					settings.palette_reindex,
					settings.sprite_reindex,
					settings.sprite_flip_h,
					settings.sprite_flip_v,
					true,
				);
			}

			if settings.export_bin_uncompressed {
				let mut file_path: PathBuf = path_buf.clone();
				file_path.push(format!("u_sprite_{:0padding$}.bin", name_index));

				Self::make_bin(
					file_path,
					&sprite.bind_mut(),
					settings.palette_include,
					settings.palette_colors.to_vec(),
					settings.palette_alpha_mode,
					settings.palette_override,
					settings.palette_reindex,
					settings.sprite_reindex,
					settings.sprite_flip_h,
					settings.sprite_flip_v,
					false,
				);
			}

			if settings.export_png {
				let mut file_path: PathBuf = path_buf.clone();
				file_path.push(format!("sprite_{:0padding$}.png", name_index));

				Self::make_png(
					file_path,
					&sprite.bind_mut(),
					settings.palette_include,
					settings.palette_colors.to_vec(),
					settings.palette_alpha_mode,
					settings.palette_override,
					settings.palette_reindex,
					settings.sprite_reindex,
					settings.sprite_flip_h,
					settings.sprite_flip_v,
				);
			}

			if settings.export_bmp {
				let mut file_path: PathBuf = path_buf.clone();
				file_path.push(format!("sprite_{:0padding$}.bmp", name_index));

				Self::make_bmp(
					file_path,
					&sprite.bind_mut(),
					settings.palette_include,
					settings.palette_colors.to_vec(),
					settings.palette_override,
					settings.palette_reindex,
					settings.sprite_reindex,
					settings.sprite_flip_h,
					settings.sprite_flip_v,
				);
			}

			if settings.export_raw {
				let file_path: PathBuf = path_buf.clone();

				Self::make_raw(
					file_path,
					format!("{:0padding$}", name_index),
					&sprite.bind_mut(),
					settings.sprite_reindex,
					settings.sprite_flip_h,
					settings.sprite_flip_v,
				);
			}
			
			name_index += 1;
		}
	}


	#[func]
	fn export_png_direct(
		path: String, sprite: Gd<BinSprite>, palette: PackedByteArray
	) {
		let path_buf: PathBuf = PathBuf::from(path);
		let mut directory: PathBuf = path_buf.clone();
		let _ = directory.pop();
		let _ = fs::create_dir_all(&directory);
		
		Self::make_png(
			path_buf,
			&sprite.bind(),
			false,				// palette_include
			palette.to_vec(),	// external_palette
			3,					// palette_alpha_mode (3 = Opaque)
			true,				// palette_override
			false,				// palette_reindex
			false,				// sprite_reindex
			false,				// sprite_flip_h
			false				// sprite_flip_v
		);
	}
}