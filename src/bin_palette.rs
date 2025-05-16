use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

use godot::prelude::*;

use crate::sprite_get;
use crate::sprite_transform;
use crate::sprite_compress::SpriteData;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Color palette obtained from loading a palette_#.bin file.
pub struct BinPalette {
	base: Base<Resource>,
	/// The color palette loaded from the file.
	#[export] pub palette: PackedByteArray,
}


#[godot_api]
impl IResource for BinPalette {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			palette: PackedByteArray::from(vec![]),
		}
	}
}


#[godot_api]
impl BinPalette {
	/// The default header to save palettes with.
	const DEFAULT_HEADER: [u8; 16] = [
		0x03, 0x00, 0x20, 0x00,
		0x08, 0x00, 0xC0, 0x00,
		0x20, 0x01, 0x08, 0x00,
		0x09, 0x00, 0xFF, 0xFF
	];
	
	
	/// Static constructor for BinPalettes from .bin files.
	#[func]
	pub fn from_bin_file(path: String) -> Option<Gd<Self>> {
		let path_buf: PathBuf = PathBuf::from(path);
		return Self::from_bin_file_pathbuf(path_buf);
	}


	pub fn from_bin_file_pathbuf(path_buf: PathBuf) -> Option<Gd<Self>> {
		if !path_buf.exists() {
			godot_print!("Could not find palette file!");
			return None;
		}
		
		match fs::read(path_buf) {
			Ok(data) => return Self::from_bin_data(data),
			
			_ => {
				godot_print!("Could not load palette file!");
				return None;
			},
		}
	}


	// Loads BinPalettes from a raw binary data vector.
	pub fn from_bin_data(bin_data: Vec<u8>) -> Option<Gd<BinPalette>> {
		// Check 'clut' byte
		if bin_data[0x02] != 0x20 {
			godot_print!("BIN data does not contain a palette.");
			return None;
		}
		
		// Get palette
		let palette: Vec<u8>;
		
		if bin_data[0x04] == 0x04 {
			palette = bin_data[0x10..0x50].to_vec();
		} else {
			palette = bin_data[0x10..0x410].to_vec();
		}
		
		return Some(
			Gd::from_init_fn(|base| {
				BinPalette {
					base,
					palette: PackedByteArray::from(palette),
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .png files.
	#[func]
	pub fn from_png_file(path: GString) -> Option<Gd<Self>> {
		let path_str: String = String::from(path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find palette file!");
			return None;
		}
		
		let sprite_data: SpriteData;
		
		match sprite_get::get_png(&path_buf) {
			None => return None,
			Some(data) => sprite_data = data,
		}
		
		if sprite_data.palette.is_empty() {
			return None;
		}
		
		return Some(
			Gd::from_init_fn(|base| {
				Self {
					base,
					palette: PackedByteArray::from(sprite_data.palette),
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .bmp files.
	#[func]
	pub fn from_bmp_file(path: GString) -> Option<Gd<Self>> {
		let path_str: String = String::from(path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find palette file!");
			return None;
		}
		
		let sprite_data: SpriteData;
		
		match sprite_get::get_bmp(&path_buf) {
			None => return None,
			Some(data) => sprite_data = data,
		}
		
		if sprite_data.palette.is_empty() {
			return None;
		}
		
		return Some(
			Gd::from_init_fn(|base| {
				Self {
					base,
					palette: PackedByteArray::from(sprite_data.palette),
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .act files.
	#[func]
	pub fn from_act_file(path: GString) -> Option<Gd<Self>> {
		let path_str: String = String::from(path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find palette file!");
			return None;
		}
		
		let act_data: Vec<u8>;
		
		match fs::read(path_buf) {
			Ok(data) => {
				if data.len() < 0x304 {
					godot_print!("Invalid .ACT file!");
					return None;
				}
				
				act_data = data;
			},
			
			_ => {
				godot_print!("Errored while reading .ACT file!");
				return None;
			}
		}
		
		// Create palette with alpha
		let mut palette: Vec<u8> = Vec::new();
		
		// Index #0
		palette.push(act_data[0]);
		palette.push(act_data[1]);
		palette.push(act_data[2]);
		palette.push(0x00);
		
		for color in 1..256 {
			palette.push(act_data[3 * color + 0]);
			palette.push(act_data[3 * color + 1]);
			palette.push(act_data[3 * color + 2]);
			palette.push(0x80);
		}
		
		return Some(
			Gd::from_init_fn(|base| {
				Self {
					base,
					palette: PackedByteArray::from(palette),
				}
			})
		);
	}
	
	
	/// Saves the palette to an .act file.
	#[func]
	pub fn to_act_file(&self, path: String) {
		let path_buf: PathBuf = PathBuf::from(path);
		let mut dir_buf: PathBuf = path_buf.clone();
		let _ = dir_buf.pop();
		let _ = fs::create_dir_all(dir_buf);
		
		let palette: Vec<u8> = self.palette.to_vec();
		let color_count: usize = palette.len() / 4;
		
		match File::create(&path_buf) {
			Ok(file) => {
				let ref mut buffer = BufWriter::new(file);
				let mut act_pal: Vec<u8> = Vec::new();
				
				for color in 0..color_count {
					act_pal.push(palette[4 * color + 0]);
					act_pal.push(palette[4 * color + 1]);
					act_pal.push(palette[4 * color + 2]);
				}
				
				act_pal.resize(256 * 3, 0u8);
				act_pal.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
				
				let _ = buffer.write_all(&act_pal);
				let _ = buffer.flush();
			},
			
			_ => (),
		}
	}
	
	
	/// Saves the palette to a .bin file.
	#[func]
	pub fn to_bin_file(&self, path: String) {
		let path_buf: PathBuf = PathBuf::from(path);
		let mut dir_buf: PathBuf = path_buf.clone();
		let _ = dir_buf.pop();
		let _ = fs::create_dir_all(dir_buf);
		
		match File::create(&path_buf) {
			Ok(file) => {
				let ref mut buffer = BufWriter::new(file);
				let _ = buffer.write_all(&self.to_bin());
				let _ = buffer.flush();
			},
			
			_ => (),
		}
	}
	
	
	pub fn to_bin(&self) -> Vec<u8> {
		let palette: Vec<u8> = self.palette.to_vec();
		let color_count: usize = palette.len() / 4;
		let mut header = Self::DEFAULT_HEADER.clone();
		
		if color_count < 17 {
			header[4] = 0x04;
		}
		
		let mut bin_data: Vec<u8> = Vec::new();
		
		bin_data.extend(header);
		bin_data.extend(palette);
		
		return bin_data;
	}
	
	
	/// Reindexing function. Reorders colors from 1-2-3-4 to 1-3-2-4 and vice versa.
	#[func]
	pub fn reindex(&mut self) {		
		let mut temp_pal: Vec<u8> = vec![0u8; self.palette.len()];
		
		let color_count: usize = self.palette.len() / 4;
		
		for color in 0..color_count {
			let new_index: usize = sprite_transform::transform_index(color as u8) as usize;
			temp_pal[4 * color + 0] = self.palette[4 * new_index + 0];
			temp_pal[4 * color + 1] = self.palette[4 * new_index + 1];
			temp_pal[4 * color + 2] = self.palette[4 * new_index + 2];
			temp_pal[4 * color + 3] = self.palette[4 * new_index + 3];
		}
		
		self.palette = PackedByteArray::from(temp_pal);
	}
	
	
	/// Alpha halving function. Halves all alpha values except for 0xFF, which is set to 0x80.
	#[func]
	pub fn alpha_halve(&mut self) {
		self.palette = sprite_transform::alpha_halve(self.palette.to_vec()).into();
	}
	
	
	/// Alpha doubling function. Doubles all alpha values except for 0x80, which is set to 0xFF.
	#[func]
	pub fn alpha_double(&mut self) {
		self.palette = sprite_transform::alpha_double(self.palette.to_vec()).into();
	}
}