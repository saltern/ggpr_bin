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
	// #[export] pub palette: PackedByteArray,
	palette: Vec<u8>,
	bit_depth: u16,
	half_size: bool,
	reindexed: bool,
}


#[godot_api]
impl IResource for BinPalette {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			palette: vec![0; 4 * 256],
			bit_depth: 8,
			half_size: false,
			reindexed: false,
		}
	}
}


#[godot_api]
impl BinPalette {
	const CHANNEL_R: usize = 0;
	const CHANNEL_G: usize = 1;
	const CHANNEL_B: usize = 2;
	const CHANNEL_A: usize = 3;
	const COLOR_SIZE: usize = 4;

	/// The default header to save palettes with.
	const DEFAULT_HEADER: [u8; 16] = [
		0x03, 0x00, 0x20, 0x00,
		0x08, 0x00, 0xC0, 0x00,
		0x20, 0x01, 0x08, 0x00,
		0x09, 0x00, 0xFF, 0xFF
	];


	#[func]
	fn get_bit_depth(&self) -> u16 {
		return self.bit_depth;
	}


	fn set_bit_depth(&mut self, value: bool) {
		if value {
			self.bit_depth = 8;
		} else {
			self.bit_depth = 4;
		}
	}


	#[func]
	fn get_reindexed(&self) -> bool {
		return self.get_bit_depth() == 8 && self.reindexed;
	}


	// Force non-reindexed on 4bpp palettes
	fn set_reindexed(&mut self, value: bool) {
		if self.get_bit_depth() == 8 {
			self.reindexed = value;
		} else {
			self.reindexed = false;
		}
	}


	fn get_index(&self, index: u8) -> usize {
		let idx: usize;

		if self.get_reindexed() {
			idx = sprite_transform::transform_index(index) as usize;
		} else {
			idx = index as usize;
		}

		return idx;
	}


	#[func]
	pub fn get_color(&self, index: u8) -> Color {
		let idx: usize = self.get_index(index);

		return Color::from_rgba8(
			self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_R],
			self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_G],
			self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_B],
			self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_A],
		);
	}


	pub fn get_vector(&self) -> Vec<u8> {
		return self.palette.clone();
	}


	#[func]
	pub fn set_color(&mut self, index: u8, color: Color) {
		let idx: usize = self.get_index(index);

		self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_R] = color.r8();
		self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_G] = color.g8();
		self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_B] = color.b8();
		self.palette[Self::COLOR_SIZE * idx + Self::CHANNEL_A] = color.a8();
	}


	#[func]
	pub fn get_half_size(&self) -> bool {
		return self.half_size;
	}


	#[func]
	pub fn set_half_size(&mut self, value: bool) {
		self.half_size = value;
	}


	pub fn from_vector(vector: Vec<u8>) -> Gd<Self> {
		let bit_depth: u16;
		let half_size: bool;
		let target_len: usize;

		if vector.len() >= 4 * 128 {
			bit_depth = 8;
		} else {
			bit_depth = 4;
		}

		// 256 colors
		if vector.len() >= Self::COLOR_SIZE * 256 {
			half_size = false;
			target_len = Self::COLOR_SIZE * 256;
		}
		// 128 colors
		else if vector.len() >= Self::COLOR_SIZE * 128 {
			half_size = true;
			target_len = Self::COLOR_SIZE * 128
		}
		// 16 colors
		else if vector.len() >= Self::COLOR_SIZE * 16 {
			half_size = false;
			target_len = Self::COLOR_SIZE * 16;
		}
		// 8 colors
		else if vector.len() >= Self::COLOR_SIZE * 8 {
			half_size = true;
			target_len = Self::COLOR_SIZE * 8;
		}
		// Some smaller, invalid value
		else {
			half_size = false;
			target_len = 0;
		}

		let palette: Vec<u8>;
		if target_len == 0 {
			palette = Vec::new();
		} else {
			palette = vector[0..target_len].to_vec();
		}

		let return_pal = Gd::from_init_fn(|base| {
			Self {
				base,
				palette,
				bit_depth,
				half_size,
				reindexed: bit_depth == 8,
			}
		});

		return return_pal;
	}


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
		// clut check
		if !(bin_data[0x02] == 0x10 ||		// half-size
			 bin_data[0x02] == 0x20 ||		// full-size
			 bin_data[0x00] == 0xFF)		// GGX palette
		{
			godot_print!("bin_palette::from_bin_data() -> BIN data does not contain a palette.");
			return None;
		}

		let half_size: bool = bin_data[0x02] == 0x10;
		let bit_depth: u16;

		// Guard rail
		match bin_data[0x04] {
			4 => bit_depth = 4,
			_ => bit_depth = 8,
		}

		let mut palette_size: usize;

		match bit_depth {
			4 => palette_size = 16,
			_ => palette_size = 256,
		}

		if half_size {
			palette_size /= 2;
		}

		let color_data_size: usize = Self::COLOR_SIZE * palette_size;

		// Get palette
		let palette: Vec<u8> = bin_data[0x10..(0x10 + color_data_size)].to_vec();
		
		return Some(
			Gd::from_init_fn(|base| {
				BinPalette {
					base,
					palette,
					bit_depth,
					half_size,
					// Not sure about this
					reindexed: bit_depth == 8,
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .png files.
	#[func]
	pub fn from_png_file(path: GString, reindexed: bool) -> Option<Gd<Self>> {
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
					palette: sprite_data.palette,
					bit_depth: sprite_data.bit_depth,
					half_size: false,
					reindexed,
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .bmp files.
	#[func]
	pub fn from_bmp_file(path: GString, reindexed: bool) -> Option<Gd<Self>> {
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
					palette: sprite_data.palette,
					bit_depth: sprite_data.bit_depth,
					half_size: false,
					reindexed,
				}
			})
		);
	}
	
	
	/// Static constructor for BinPalettes from .act files.
	#[func]
	pub fn from_act_file(path: GString, half_size: bool, bit_depth: u16, reindexed: bool) -> Option<Gd<Self>> {
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
		palette.push(act_data[Self::CHANNEL_R]);
		palette.push(act_data[Self::CHANNEL_G]);
		palette.push(act_data[Self::CHANNEL_B]);
		palette.push(0x00);
		
		for color in 1..256 {
			palette.push(act_data[3 * color + Self::CHANNEL_R]);
			palette.push(act_data[3 * color + Self::CHANNEL_G]);
			palette.push(act_data[3 * color + Self::CHANNEL_B]);
			palette.push(0x80);
		}
		
		return Some(
			Gd::from_init_fn(|base| {
				Self {
					base,
					palette,
					bit_depth,
					half_size,
					reindexed,
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
					act_pal.push(palette[4 * color + Self::CHANNEL_R]);
					act_pal.push(palette[4 * color + Self::CHANNEL_G]);
					act_pal.push(palette[4 * color + Self::CHANNEL_B]);
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
		if self.get_bit_depth() == 8 {
			self.palette = sprite_transform::reindex_rgba_vector(self.palette.clone());
		}
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