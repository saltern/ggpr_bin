use std::fs;
use std::path::PathBuf;
use std::ops::Deref;
use std::cmp;
use std::ffi::OsStr;

use crate::bin_cell::Cell;
use crate::bin_sprite::BinSprite;
use crate::bin_palette::BinPalette;
use crate::sprite_load_save::SpriteLoadSave;

use godot::prelude::*;

/*
		type: full_object,
		type: sprite_list,
 */


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Holds all of the data pertaining to a single object from a binary file.
pub struct ObjectData {
	pub base: Base<Resource>,
	#[export] pub name: GString,
	#[export] pub cells: Array<Gd<Cell>>,
	#[export] pub sprites: Array<Gd<BinSprite>>,
	#[export] pub script: PackedByteArray,
	#[export] pub palettes: Array<Gd<BinPalette>>,
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
			palettes: Array::new(),
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
	pub fn save_as_directory(&self, path: String) {
		self.save_cells_to_path(&path);
		self.save_sprites_to_path(&path);
		self.save_palettes_to_path(&path);
	}
	
	
	/// Saves cells to a path in JSON format.
	fn save_cells_to_path(&self, path: &String) {
		if self.cells.len() < 1 {
			return;
		}
		
		fs::create_dir_all(format!("{}/cells_0", path)).unwrap();
		
		for cell_number in 0..self.cells.len() {
			let item = self.cells.at(cell_number);
			let cell = item.bind();
			cell.to_file(PathBuf::from(format!("{}/cells_0/cell_{}.json", path, cell_number)));
		}
		
		// Replace old files with new
		let _ = fs::rename(format!("{}/cells_0", path), format!("{}/cells_1", path));
		let _ = fs::remove_dir_all(format!("{}/cells", path));
		let _ = fs::rename(format!("{}/cells_1", path), format!("{}/cells", path));
	}
	
	
	/// Saves sprites to a path in compressed .bin format.
	fn save_sprites_to_path(&self, path: &String) {
		fs::create_dir_all(format!("{}/sprites_0", path)).unwrap();
		
		SpriteLoadSave::save_sprites(self.sprites.clone(), format!("{}/sprites_0", &path));
		
		// Replace old files with new
		let _ = fs::rename(format!("{}/sprites_0", &path), format!("{}/sprites_1", &path));
		let _ = fs::remove_dir_all(format!("{}/sprites", path));
		let _ = fs::rename(format!("{}/sprites_1", &path), format!("{}/sprites", &path));
	}
	
	
	/// Saves palettes to a path in .bin format.
	fn save_palettes_to_path(&self, path: &String) {
		if self.palettes.len() < 1 {
			return;
		}
		
		let mut palette_number: usize = 0;
		
		for item in self.palettes.iter_shared() {
			let mut path_buf: PathBuf = PathBuf::from(path);
			path_buf.pop();
			path_buf.push(format!("palettes/pal_{}.bin", palette_number));
			
			let binding = item.bind();
			let palette: &BinPalette = binding.deref();
			palette.to_bin_file(path_buf);
			palette_number += 1;
		}
	}
	
	
	pub fn get_as_binary(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();
		
		bin_data.extend(self.get_binary_cells());
		bin_data.extend(self.get_binary_sprites());
		bin_data.extend(self.script.to_vec());
		bin_data.extend(self.get_binary_palettes());
		return bin_data;
	}
	
	
	fn get_binary_cells(&self) -> Vec<u8> {
		let mut vector_cells: Vec<u8> = Vec::new();
		let mut vector_offsets: Vec<u32> = Vec::new();
		let mut cursor_offsets: u32 = 0x00;
		
		for cell in self.cells.iter_shared() {
			let cell_bin: Vec<u8> = cell.bind().to_bin();
			
			vector_offsets.push(cursor_offsets);
			cursor_offsets += cell_bin.len() as u32;
			
			vector_cells.extend(cell_bin);
		}
		
		vector_offsets.resize(
			vector_offsets.len() + vector_offsets.len() % 0x04, 0xFFFFFFFF
		);
		
		let mut vector_full: Vec<u8> = Vec::new();
		
		// Register cell pointers
		for offset in vector_offsets.iter() {
			vector_full.extend((offset + vector_offsets.len() as u32).to_le_bytes());
		}
		
		vector_full.extend(vector_cells);
		return vector_full;
	}
	
	
	fn get_binary_sprites(&self) -> Vec<u8> {
		let mut vector_sprites: Vec<u8> = Vec::new();
		let mut vector_offsets: Vec<u32> = Vec::new();
		let mut cursor_offsets: u32 = 0x00;
		
		for sprite in self.sprites.iter_shared() {
			let sprite_bin: Vec<u8> = sprite.bind().to_bin();
			
			vector_offsets.push(cursor_offsets);
			cursor_offsets += sprite_bin.len() as u32;
			
			vector_sprites.extend(sprite_bin);
			
		}
		
		vector_offsets.resize(
			vector_offsets.len() + vector_offsets.len() % 0x04, 0xFFFFFFFF
		);
		
		let mut vector_full: Vec<u8> = Vec::new();
		
		// Register sprite pointers
		for offset in vector_offsets.iter() {
			vector_full.extend((offset + vector_offsets.len() as u32).to_le_bytes());
		}
		
		vector_full.extend(vector_sprites);
		return vector_full;
	}
	
	
	fn get_binary_palettes(&self) -> Vec<u8> {
		let mut vector_palettes: Vec<u8> = Vec::new();
		let mut vector_offsets: Vec<u32> = Vec::new();
		let mut cursor_offsets: u32 = 0x00;
		
		for palette in self.palettes.iter_shared() {
			let palette_bin: Vec<u8> = palette.bind().to_bin();
			
			vector_offsets.push(cursor_offsets);
			cursor_offsets += palette_bin.len() as u32;
			
			vector_palettes.extend(palette_bin);
		}
		
		vector_offsets.resize(
			vector_offsets.len() + vector_offsets.len() % 0x04, 0xFFFFFFFF
		);
		
		let mut vector_full: Vec<u8> = Vec::new();
		
		// Register palette pointers
		for offset in vector_offsets.iter() {
			vector_full.extend((offset + vector_offsets.len() as u32).to_le_bytes());
		}
		
		vector_full.extend(vector_palettes);
		return vector_full;
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
			self.load_palettes_from_path(format!("{}/../palettes", path));
		}
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
	
	
	// Loads .bin sprites from a path.
	fn load_sprites_from_path(&mut self, path: String) -> bool {
		let path_buf = PathBuf::from(path.clone());
		
		if !path_buf.exists() {
			return false;
		}
		
		self.sprites = SpriteLoadSave::load_sprites(path);
		return true;
	}
	
	
	/// Loads all .bin palettes in a directory to memory.
	pub fn load_palettes_from_path(&mut self, path: String) {
		self.palettes.clear();
		
		let path_buf: PathBuf = PathBuf::from(String::from(path));
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
			match fs::read(file) {
				Ok(data) => match BinPalette::from_bin_data(data) {
					Some(palette) => self.palettes.push(&palette),
					_ => continue,
				},
				
				_ => continue,
			}
		}
	}
	
	
	// =================================================================================
	// CELLS
	// =================================================================================
	
	
	/// Returns whether this object has cells or not.
	#[func]
	pub fn has_cells(&self) -> bool {
		return self.cells.len() > 0;
	}
	
	
	/// Returns cell numbers affected by sprite index clamping.
	#[func]
	pub fn clamp_get_affected_cells(&self, sprite_max: u16) -> PackedInt64Array {
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
	pub fn clamp_sprite_indices(&self) {
		let sprite_max: u16 = cmp::max(self.sprites.len() - 1, 0) as u16;
		
		for mut cell in self.cells.iter_shared() {
			let mut binding = cell.bind_mut();
			binding.clamp_sprite_index(sprite_max);
		}
	}
	
	
	/// Returns cells affected by sprite index redirection.
	#[func]
	pub fn redirect_get_affected_cells(&self, from: u16) -> PackedInt64Array {
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
	pub fn redirect_sprite_indices(&self, from: u16, how_many: u16) {
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
	
	
	// =================================================================================
	// SPRITES
	// =================================================================================
	
	
	/// Returns a BinSprite from this object, or a blank one if out of bounds.
	#[func]
	pub fn sprite_get(&self, index: u16) -> Gd<BinSprite> {
		if self.sprites.len() > index as usize {
			return self.sprites.at(index as usize);
		}
		
		else {
			return BinSprite::new_gd();
		}
	}
	
	
	/// Returns the number of sprites contained in this object.
	#[func]
	pub fn sprite_get_count(&self) -> i64 {
		return self.sprites.len() as i64;
	}
	
	
	// =================================================================================
	// SCRIPT
	// =================================================================================
	
	
	// ...
	
	
	// =================================================================================
	// PALETTES
	// =================================================================================
	
	
	/// Returns whether this object has palettes or not.
	#[func]
	pub fn has_palettes(&self) -> bool {
		return self.palettes.len() > 0;
	}
	
	
	/// Returns a BinPalette from this object, or a blank one if out of bounds.
	#[func]
	pub fn palette_get(&self, index: i64) -> Gd<BinPalette> {
		if self.palettes.len() > index as usize {
			return self.palettes.at(index as usize);
		}
		
		else {
			return BinPalette::new_gd();
		}
	}
	
	
	/// Returns the number of palettes contained in this object.
	#[func]
	pub fn palette_get_count(&self) -> i64 {
		return self.palettes.len() as i64;
	}
	
	
	/// Emits the signal that indicates that a palette was selected.
	#[func]
	pub fn palette_broadcast(&mut self, index: i64) {
		self.base_mut().emit_signal("palette_selected", &[index.to_variant()]);
	}
}