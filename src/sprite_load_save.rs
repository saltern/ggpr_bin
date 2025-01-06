use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

use godot::prelude::*;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::bin_sprite;
use crate::sprite_get;
use crate::sprite_compress;

use bin_sprite::BinSprite;
use sprite_compress::SpriteData;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite decompressor and loader, based on Ghoul.
pub struct SpriteLoadSave {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for SpriteLoadSave {
	fn init(base: Base<Resource>) -> Self {
		Self { base }
	}
}


#[godot_api]
impl SpriteLoadSave {
	/// Loads BIN sprites in natural naming order from a specified path.
	#[func]
	pub fn load_sprites(source_path: String) -> Array<Gd<BinSprite>> {
		let path_buf: PathBuf = PathBuf::from(source_path);
		return Self::load_sprites_pathbuf(path_buf);
	}
	
	
	pub fn load_sprites_pathbuf(path_buf: PathBuf) -> Array<Gd<BinSprite>> {
		if !path_buf.exists() {
			godot_print!("Could not find sprite directory!");
			godot_print!("Provided path: {}", path_buf.display());
			return array![];
		}
		
		let mut file_vector: Vec<PathBuf> = Vec::new();
		
		match fs::read_dir(path_buf) {
			Ok(value) => {
				for entry in value {
					file_vector.push(entry.unwrap().path());
				}
			},
			
			_ => return array![],
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
		let path_buf: PathBuf = PathBuf::from(target_path);
		
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
					godot_print!(
						"sprite_load_save::save_sprites() error: Could not create target file!"
					);
					godot_print!("\tFile: '{:?}'", &target_file);
					continue;
				}
			}
			
			let ref mut buffer = BufWriter::new(bin_file);
			
			// OK to write
			
			// Get data
			let binding = gd_sprite.bind();
			buffer.write_all(binding.to_bin().as_slice()).unwrap();
			let _ = buffer.flush();
			
			sprite_number += 1;
		}
	}
}