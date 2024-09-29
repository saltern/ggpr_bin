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
use crate::sprite_compress;
use crate::sprite_transform;

use bin_sprite::BinSprite;
use bin_sprite::BinHeader;
use sprite_compress::SpriteData;
use sprite_compress::CompressedData;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R sprite decompressor and loader, based on Ghoul.
struct SpriteLoadSave {
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
	fn load_sprites(source_path: GString) -> Array<Gd<BinSprite>> {
		let path_str: String = String::from(source_path);
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
				sprite_data = sprite_compress::decompress(bin_data, bin_header);
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
					byte_vector = sprite_transform::bpp_from_4(byte_vector, true);
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


	/// Saves BIN sprites to a specified path. Overwrites existing files.
	#[func]
	fn save_sprites(sprites: Array<Gd<BinSprite>>, target_path: GString) {
		let path_str: String = String::from(target_path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find sprite directory!");
			return Default::default();
		}
		
		// Clear out target path first. Scary!
		let _ = fs::remove_dir_all(&path_buf).and_then(|_| fs::create_dir(&path_buf));
		
		let mut sprite_number: usize = 0;
		
		for mut gd_sprite in sprites.iter_shared() {
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
			let binding = gd_sprite.bind_mut();
			let sprite: &BinSprite = binding.deref();
			
			let image: &Image = sprite.image.as_ref().unwrap();
			let tex_width: u16 = image.get_width() as u16;
			let tex_height: u16 = image.get_height() as u16;
			
			let pixels: Vec<u8> = sprite.pixels.to_vec();
			
			let sprite_data: SpriteData = SpriteData {
				width: tex_width,
				height: tex_height,
				bit_depth: sprite.bit_depth,
				pixels: pixels,
				palette: sprite.palette.to_vec(),
			};
			
			let clut: u16;
			if sprite_data.palette.is_empty() { clut = 0x0000; } else { clut = 0x0020; }
			
			// Get bytes
			let header_bytes: Vec<u8> = bin_sprite::make_header(
				true,				// compressed
				clut,				// embedded palette yes/no
				sprite.bit_depth,	// bit depth
				tex_width,			// sprite width
				tex_height,			// sprite height
				0x0000,				// tw
				0x0000,				// th
				0x0000				// hash
			);
			
			// Write header
			let _ = buffer.write_all(&header_bytes);
			
			// Write palette
			let _ = buffer.write_all(sprite_data.palette.as_slice());
			
			// Compress
			let compressed_data: CompressedData = sprite_compress::compress(sprite_data);
			
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