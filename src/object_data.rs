use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::ops::DerefMut;
use std::cmp;
use std::ffi::OsStr;

use crate::bin_cell::Cell;
use crate::bin_sprite::BinSprite;
use crate::bin_palette;
use crate::bin_palette::BinPalette;
use crate::sprite_load_save::SpriteLoadSave;

use godot::prelude::*;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Holds the player object's palettes.
pub struct PaletteData {
	base: Base<Resource>,
	#[export] palettes: Array<Gd<BinPalette>>,
}


#[godot_api]
impl IResource for PaletteData {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
			palettes: Array::new(),
		}
	}
}


#[godot_api]
impl PaletteData {
	/// The default header to save palettes with.
	const DEFAULT_HEADER: &[u8; 16] = &[
		0x03, 0x00, 0x20, 0x00,
		0x08, 0x00, 0xC0, 0x00,
		0x20, 0x01, 0x08, 0x00,
		0x09, 0x00, 0xFF, 0xFF
	];
	
	
	/// Saves a single palette to a file in .act or .bin format.
	#[func]
	pub fn save_palette(pal_array: PackedByteArray, path: GString) {
		let path_buf: PathBuf = PathBuf::from(String::from(path));
		let palette: Vec<u8> = pal_array.to_vec();
		let color_count: usize = palette.len() / 4;
		
		match File::create(&path_buf) {
			Ok(file) => {
				let ref mut buffer = BufWriter::new(file);
				
				match path_buf.extension().unwrap().to_str() {
					Some("act") => {
						let mut act_pal: Vec<u8> = Vec::new();
						
						for color in 0..color_count {
							act_pal.push(palette[4 * color + 0]);
							act_pal.push(palette[4 * color + 1]);
							act_pal.push(palette[4 * color + 2]);
						}
						
						act_pal.resize(768, 0u8);
						act_pal.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
						
						let _ = buffer.write_all(&act_pal);
					},
					
					_ => {
						let mut header: [u8; 16] = Self::DEFAULT_HEADER.clone();
						
						if color_count == 16 {
							header[4] = 0x04;
						}
						
						let _ = buffer.write_all(&header);
						let _ = buffer.write_all(&palette);
					},
				}
				
				let _ = buffer.flush();
			},
				
			_ => return,
		}
	}
	
	
	/// Saves all palettes in memory to .bin files.
	pub fn serialize_and_save(&mut self, mut path_buf: PathBuf) {
		path_buf.pop();
		path_buf.push("/palettes");
		
		for palette_number in 0..self.palettes.len() {
			let palette: PackedByteArray = self.palettes.at(palette_number).bind().palette.clone();
			let mut pal_path: PathBuf = path_buf.clone();
			pal_path.push(format!("palette_{}.bin", palette_number));
			
			match File::create(&pal_path) {
				Ok(file) => {
					let ref mut buffer = BufWriter::new(file);
					let _ = buffer.write_all(Self::DEFAULT_HEADER);
					let _ = buffer.write_all(&(palette.to_vec()));
					let _ = buffer.flush();
				},
				
				_ => continue,
			}
		}
	}
	
	
	/// Loads all .bin palettes in a directory to memory.
	pub fn load_palettes_from_path(&mut self, source_path: String) {
		self.palettes.clear();
		
		let path_buf: PathBuf = PathBuf::from(String::from(source_path));
		if !path_buf.exists() {
			return;
		}
		
		let mut file_vector: Vec<PathBuf> = Vec::new();
		
		match fs::read_dir(path_buf) {
			Ok(entries) => {
				for entry in entries {
					file_vector.push(entry.unwrap().path());
				}
			},
			
			_ => return,
		}
		
		file_vector.sort_by(|a, b| natord::compare(a.to_str().unwrap(), b.to_str().unwrap()));
		
		for file in file_vector {
			match Self::load_palette_file(file) {
				Some(palette) => self.palettes.push(&palette),
				_ => continue,
			}
		}
	}
	
	
	// Part of the loading function that deals with each file.
	pub fn load_palette_file(file: PathBuf) -> Option<Gd<BinPalette>> {
		match file.extension() {
			Some(os_str) => {
				if os_str.to_ascii_lowercase().to_str() != Some("bin") {
					return None;
				}
			},
			
			_ => return None,
		}
		
		match fs::read(file) {
			Ok(bin_data) => return bin_palette::from_bin_data(bin_data),
			_ => return None,
		}
	}
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Holds all of the data pertaining to a single object from a binary file.
pub struct ObjectData {
	base: Base<Resource>,
	#[var] pub name: GString,
	#[var] pub cells: Array<Gd<Cell>>,
	#[var] pub sprites: Array<Gd<BinSprite>>,
	#[var] pub script: PackedByteArray,
	#[var] pub palette_data: Gd<PaletteData>,
	//#[var] pub object_script: Gd<ObjectScript>,
}


#[godot_api]
impl IResource for ObjectData {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
			name: "".into(),
			cells: array![],
			sprites: array![],
			script: PackedByteArray::new(),
			palette_data: PaletteData::new_gd(),
		}
	}
}


#[godot_api]
impl ObjectData {
	/// Signals changes to a palette. Emitted by [PaletteProvider] when a color is changed. Used by [SpriteEdit].
	#[signal] fn palette_updated();
	
	/// Signals that a specific palette was selected. Used to sync the palette across editors.
	#[signal] fn palette_selected(palette_index: i64);
	
	/// Signals that this object has started saving.
	#[signal] fn saving_element();
	
	
	// =================================================================================
	// SAVING (DIRECTORY)
	// =================================================================================
	
	
	/// Calls serialization functions for cell, sprite, and palette data, saving the results to the specified path.
	#[func]
	pub fn save_as_directory(&mut self, path: String) {
		self.save_cells_to_path(path.clone());
		self.save_sprites_to_path(path.clone());
		self.palette_data.bind_mut().serialize_and_save(PathBuf::from(path));
	}
	
	
	/// Returns a string array of JSON-serialized cells.
	fn serialize_cells(&mut self) -> Array<GString> {
		let mut array: Array<GString> = array![];
		
		for mut cell in self.cells.iter_shared() {
			let mut cell_bind = cell.bind_mut();
			array.push(&cell_bind.deref_mut().serialize());
		}
		
		return array;
	}
	
	
	/// Saves cells to a path in JSON format.
	fn save_cells_to_path(&mut self, path: String) {
		if self.cells.len() < 1 {
			return;
		}
		
		fs::create_dir_all(format!("{}/cells_0", path)).unwrap();
		
		let mut cell_num: usize = 0;
		
		for cell in self.serialize_cells().iter_shared() {
			match fs::File::create(format!("{}/cells_0/cell_{}.json", path, cell_num)) {
				Ok(file) => {
					let ref mut buffer = BufWriter::new(file);
					cell_num += 1;
					
					let _ = buffer.write(String::from(cell).as_bytes());
					let _ = buffer.flush();
				},
				
				// Fail
				_ => return,
			}
		}
		
		// Replace old files with new
		let _ = fs::rename(format!("{}/cells_0", path), format!("{}/cells_1", path));
		let _ = fs::remove_dir_all(format!("{}/cells", path));
		let _ = fs::rename(format!("{}/cells_1", path), format!("{}/cells", path));
	}
	
	
	/// Saves sprites to a path in compressed .bin format.
	fn save_sprites_to_path(&mut self, path: String) {
		fs::create_dir_all(format!("{}/sprites_0", path)).unwrap();
		
		SpriteLoadSave::save_sprites(self.sprites.clone(), format!("{}/sprites_0", &path));
		
		// Replace old files with new
		let _ = fs::rename(format!("{}/sprites_0", &path), format!("{}/sprites_1", &path));
		let _ = fs::remove_dir_all(format!("{}/sprites", path));
		let _ = fs::rename(format!("{}/sprites_1", &path), format!("{}/sprites", &path));
	}
	
	
	// =================================================================================
	// LOADING (DIRECTORY)
	// =================================================================================
	
	
	/// Loads data (cells, sprites, palettes) from a path.
	#[func]
	pub fn load_data_from_path(&mut self, path: String) {
		let path_buf = PathBuf::from(&path);
		if !path_buf.exists() {
			return;
		}
		
		self.load_sprites_from_path(format!("{}/sprites", path));
		
		// Fail early
		if self.sprites.len() < 1 {
			return;
		}
		
		self.load_cells_from_path(format!("{}/cells", path));
		
		// Palettes
		if path_buf.file_name() == Some(OsStr::new("player")) {
			self.load_palette_data_from_path(format!("{}/../palettes", path));
		}
	}
	
	
	// Loads .bin sprites from a path.
	fn load_sprites_from_path(&mut self, path: String) -> bool {
		let path_buf = PathBuf::from(path.clone());
		
		if !path_buf.exists() {
			return false;
		}
		
		self.sprites = SpriteLoadSave::load_sprites(path);
		return true;
	}
	
	
	// Loads .json cells from a path.
	fn load_cells_from_path(&mut self, path: String) -> bool {
		let path_buf = PathBuf::from(path);
		if !path_buf.exists() {
			return false;
		}
		
		let mut file_vector: Vec<PathBuf> = Vec::new();
		
		match fs::read_dir(path_buf) {
			Ok(entries) => {
				for entry in entries {
					file_vector.push(entry.unwrap().path());
				}
			},
			
			_ => return false,
		}
		
		file_vector.sort_by(|a, b| natord::compare(a.to_str().unwrap(), b.to_str().unwrap()));
		
		for file in file_vector {
			match Cell::from_file(file) {
				Some(cell) => self.cells.push(&cell),
				_ => continue,
			}
		}
		
		return true;
	}
	
	
	// Loads any palettes present in a path.
	fn load_palette_data_from_path(&mut self, path: String) -> bool {
		let mut binding = self.palette_data.bind_mut();
		let palette_data = binding.deref_mut();
		palette_data.load_palettes_from_path(path);
		return palette_data.palettes.len() > 0;
	}
	
	
	// =================================================================================
	// UTILITY
	// =================================================================================
	
	
	/// Returns cell numbers affected by sprite index clamping.
	#[func]
	pub fn clamp_get_affected_cells(&mut self, sprite_max: u16) -> PackedInt64Array {
		let mut return_array = PackedInt64Array::new();
		
		for cell_number in 0..self.cells.len() {
			let item = self.cells.at(cell_number);
			let binding = item.bind();
			
			if binding.sprite_index > sprite_max {
				return_array.push(cell_number as i64);
			}
		}
		
		return return_array;
	}
	
	
	/// Clamps all cells' sprite indices so they stay within available sprites.
	#[func]
	pub fn clamp_sprite_indices(&mut self) {
		let sprite_max: u16 = cmp::max(self.sprites.len() - 1, 0) as u16;
		
		for mut item in self.cells.iter_shared() {
			let mut cell = item.bind_mut();
			cell.sprite_index = cell.sprite_index.clamp(0, sprite_max);
		}
	}
	
	
	/// Returns cells affected by sprite index redirection.
	#[func]
	pub fn redirect_get_affected_cells(&mut self, from: u16) -> PackedInt64Array {
		let mut return_array = PackedInt64Array::new();
		
		for cell_number in 0..self.cells.len() {
			let item = self.cells.at(cell_number);
			let cell = item.bind();
			
			if cell.sprite_index >= from {
				return_array.push(cell_number as i64);
			}
		}
		
		return return_array;
	}
	
	
	/// Redirects cells' sprite indices after deleting sprites.
	#[func]
	pub fn redirect_sprite_indices(&mut self, from: u16, how_many: u16) {
		let to: u16 = from + how_many - 1;
		
		for mut item in self.cells.iter_shared() {
			let mut cell = item.bind_mut();
			
			if cell.sprite_index < from {
				continue;
			}
			
			else if cell.sprite_index <= to {
				cell.sprite_index = 0;
			}
			
			else {
				cell.sprite_index -= how_many;
			}
		}
	}
	
	
	/// Returns a BinSprite from this object, or a blank one if out of bounds.
	#[func]
	pub fn sprite_get(&mut self, index: u16) -> Gd<BinSprite> {
		if self.sprites.len() > index as usize {
			return self.sprites.at(index as usize);
		}
		
		else {
			return BinSprite::new_gd();
		}
	}
	
	
	/// Returns the number of sprites contained in this object.
	#[func]
	pub fn sprite_get_count(&mut self) -> i64 {
		return self.sprites.len() as i64;
	}
	
	
	/// Returns whether this object has palettes or not.
	#[func]
	pub fn has_palettes(&mut self) -> bool {
		return self.palette_data.bind().palettes.len() > 0;
	}
	
	
	/// Returns a BinPalette from this object, or a blank one if out of bounds.
	#[func]
	pub fn palette_get(&mut self, index: i64) -> Gd<BinPalette> {
		if self.palette_data.bind().palettes.len() > index as usize {
			return self.palette_data.bind().palettes.at(index as usize);
		}
		
		else {
			return BinPalette::new_gd();
		}
	}
	
	
	/// Returns the number of palettes contained in this object.
	#[func]
	pub fn palette_get_count(&mut self) -> i64 {
		return self.palette_data.bind().palettes.len() as i64;
	}
	
	
	/// Emits the signal that indicates that a palette was selected.
	#[func]
	pub fn palette_broadcast(&mut self, index: i64) {
		self.base_mut().emit_signal("palette_selected", &[index.to_variant()]);
	}
	
	
	/// Returns whether this object has cells or not.
	#[func]
	pub fn has_cells(&mut self) -> bool {
		return self.cells.len() > 0;
	}
}