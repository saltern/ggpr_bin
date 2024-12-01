use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::ops::Deref;

use godot::prelude::*;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::bin_sprite;
use crate::sprite_get;
use crate::sprite_compress;

use bin_sprite::BinSprite;
use sprite_compress::SpriteData;
use sprite_compress::CompressedData;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite decompressor and loader, based on Ghoul.
pub struct SpriteLoadSave {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for SpriteLoadSave {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl SpriteLoadSave {
	/// Loads BIN sprites in natural naming order from a specified path.
	#[func]
	pub fn load_sprites(source_path: String) -> Array<Gd<BinSprite>> {
		let path_buf: PathBuf = PathBuf::from(source_path);
		
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
			match Self::load_sprite_file(item) {
				Some(sprite) => sprite_vector.push(&sprite),
				None => continue,
			}
		}
		
		return sprite_vector;
	}
	
	// Part of the loading function that deals with a sprite stored in the file system.
	fn load_sprite_file(file: PathBuf) -> Option<Gd<BinSprite>> {
		match file.extension() {
			Some(os_str) => {
				if os_str.to_ascii_lowercase().to_str() != Some("bin") {
					return None;
				}
			},
			
			_ => return None,
		}
	
		match fs::read(file) {
			Ok(data) => return Self::load_sprite_data(&data),
			_ => return None,
		}
	}
	
	
	// Loads BinSprites from a raw binary data vector.
	pub fn load_sprite_data(bin_data: &Vec<u8>) -> Option<Gd<BinSprite>> {
		let sprite_data: SpriteData;

		match sprite_get::get_bin_data(&bin_data) {
			None => return None,
			Some(data) => {
				if data.width == 0 || data.height == 0 {
					return None;
				}
				
				sprite_data = data;
			}
		}
		
		let sprite_image = Image::create_from_data(
			// Dimensions
			sprite_data.width as i32,
			sprite_data.height as i32,
			// Mipmapping
			false,
			// Grayscale format
			Format::L8,
			// Pixel array
			&PackedByteArray::from(sprite_data.pixels.clone())
		);
		
		match sprite_image {
			Some(image) => return Some(BinSprite::new_from_data(
				PackedByteArray::from(sprite_data.pixels),
				image,
				sprite_data.bit_depth,
				PackedByteArray::from(sprite_data.palette)
			)),
			
			_ => {
				return None;
			}
		}
	}
	

	/// Saves BIN sprites to a specified path. Overwrites existing files.
	#[func]
	pub fn save_sprites(sprites: Array<Gd<BinSprite>>, target_path: String) {
		let path_str: String = String::from(target_path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find sprite directory!");
			return Default::default();
		}
		
		let mut sprite_number: usize = 0;

		for gd_sprite in sprites.iter_shared() {
			// Create file
			let mut target_file: PathBuf = path_buf.clone();
			target_file.push(format!("sprite_{}.bin", sprite_number));
			
			let bin_file: File;
			
			match File::create(&target_file) {
				Ok(file) => bin_file = file,
				_ => {
					sprite_number += 1;
					godot_print!("sprite_load_save::save_sprites() error: Could not create target file!");
					godot_print!("\tFile: '{:?}'", &target_file);
					continue;
				}
			}
			
			let ref mut buffer = BufWriter::new(bin_file);
			
			// OK to write
			
			// Get data
			let binding = gd_sprite.bind();
			let sprite: &BinSprite = binding.deref();
			
			let image: &Image = sprite.image.as_ref().unwrap();
			let tex_width: u16 = image.get_width() as u16;
			let tex_height: u16 = image.get_height() as u16;
			
			let pixels: Vec<u8> = sprite.pixels.to_vec();
			let palette: Vec<u8> = sprite.palette.to_vec();
			
			let sprite_data: SpriteData = SpriteData {
				width: tex_width,
				height: tex_height,
				bit_depth: sprite.bit_depth,
				pixels: pixels,
				palette: palette,
			};
			
			let clut: u16;
			if sprite_data.palette.is_empty() { clut = 0x0000; } else { clut = 0x0020; }
			let palette_clone: Vec<u8> = sprite_data.palette.clone();
			let palette_slice: &[u8] = palette_clone.as_slice();
			
			// Compress
			let compressed_data: CompressedData = sprite_compress::compress(sprite_data);
	
			// Generate hash
			let mut hash: u16 = 0;
			
			for byte in 0..compressed_data.stream.len() / 2 {
				hash = hash ^ (compressed_data.stream[byte] as u16 | (compressed_data.stream[byte + 1] as u16) << 8);
			}
			
			// Get bytes
			let header_bytes: Vec<u8> = bin_sprite::make_header(
				true,				// compressed
				clut,				// embedded palette yes/no
				sprite.bit_depth,	// bit depth
				tex_width,			// sprite width
				tex_height,			// sprite height
				0x0000,				// tw
				0x0000,				// th
				hash				// hash
			);
			
			// Write header
			let _ = buffer.write_all(&header_bytes);
			
			// Write palette
			let _ = buffer.write_all(palette_slice);
			
			// Write iterations
			let iterations_u32: u32 = compressed_data.iterations as u32;
			let _ = buffer.write_all(&[
				(iterations_u32 >> 16) as u8,	// BB
				(iterations_u32 >> 24) as u8,	// AA
				(iterations_u32 >> 00) as u8,	// DD
				(iterations_u32 >> 08) as u8,	// CC
			]);
			
			// Write sprite data (LE 16 bit)
			let mut byte: usize = 0;
			let length: usize = compressed_data.stream.len();
			
			while byte + 1 < length {
				let _ = buffer.write_all(&[
					compressed_data.stream[byte + 1],
					compressed_data.stream[byte + 0],
				]);
				
				byte += 2;
			}
			
			let _ = buffer.flush();
			
			sprite_number += 1;
		}
	}
}