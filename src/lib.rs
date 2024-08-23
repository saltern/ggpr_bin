use std::path::PathBuf;
use std::fs;

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
	fn load_sprites(g_path: GString) -> Array<Gd<BinSprite>> {
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
		// let mut texture_vector: Array<Gd<ImageTexture>> = array![];
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
				sprite_data = decompressor::decompress(bin_data, bin_header);
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
				
				if bin_header.bit_depth == 4 {
					byte_vector = sprite_transform::bpp_from_4(byte_vector);
				}
				
				// Truncate
				byte_vector.resize((bin_header.width * bin_header.height) as usize, 0u8);
				
				sprite_data = SpriteData {
					width: bin_header.width,
					height: bin_header.height,
					pixels: byte_vector,
					palette: palette,
				}
			}
			
			let image: Gd<Image>;
			
			match Image::create_from_data(sprite_data.width as i32, sprite_data.height as i32, false, Format::L8, PackedByteArray::from(sprite_data.pixels)) {
				Some(gd_image) => image = gd_image,
				_ => continue,
			}
			
			// texture_vector.push(ImageTexture::create_from_image(image).unwrap());
			sprite_vector.push(BinSprite::new_from_data(ImageTexture::create_from_image(image).unwrap(), PackedByteArray::from(sprite_data.palette)));
		}
		
		// return texture_vector;
		return sprite_vector;
	}
}