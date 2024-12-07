use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::path::PathBuf;

use godot::prelude::*;

use crate::bin_identify::*;
use crate::bin_sprite::BinSprite;
use crate::bin_cell::Cell;
use crate::bin_palette::BinPalette;
use crate::sprite_load_save::SpriteLoadSave;

/* object_type:
 *		"sprite"				<- single sprite
 *		"sprite_list_select"	<- pointers to individual sprites, then the select screen cursor mask
 *		"sprite_list"			<- pointers to individual sprites
 *		"jpf_plain_text"		<- char_index.bin, then individual sprites
 *		"scriptable"			<- cells, sprites, script, and possibly palettes
 *		"wii_tpl"				<- Wii TPL texture (not currently in use)
 *		"multi_object"			<- contains scriptable subobjects (archive_jpf.bin effects)
 *		"dummy"					<- "DUMMY" padding (not currently in use)
 *		"unsupported"			<- as-is binary passthrough
 */


struct Scriptable {
	name: String,
	cells: Array<Gd<Cell>>,
	sprites: Array<Gd<BinSprite>>,
	scripts: PackedByteArray,
	palettes: Array<Gd<BinPalette>>,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Representation of a single binary resource file.
struct BinResource {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for BinResource {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl BinResource {
	/// Loads a BIN resource file, returning the objects contained within.
	#[func] fn from_file(source_path: String) -> Dictionary {
		let path_buf: PathBuf = PathBuf::from(&source_path);
		
		godot_print!("Loading {}...", &source_path);
		
		if !path_buf.exists() {
			godot_print!("File was not found");
			return dict! {
				"error": "File not found",
			};
		}
		
		match fs::read(path_buf) {
			Ok(data) => return Self::load_binary_data(data),
			
			_ => return dict! {
				"error": "Could not read file",
			},
		}
	}


	fn load_binary_data(bin_data: Vec<u8>) -> Dictionary {
		let data_length: usize = bin_data.len();
		
		// Smallest possible file is a SpriteList with a single, palette-less 1x1 sprite
		// Such a file is 48 bytes long (0x30 hex)
		if data_length < 0x30 {
			return dict! {
				"error": "Invalid file (too short)",
			}
		}
		
		if u32::from_le_bytes([
			bin_data[data_length - 0x01], bin_data[data_length - 0x02],
			bin_data[data_length - 0x03], bin_data[data_length - 0x04],
		]) == ENCRYPTED_SIGNATURE {
			return dict! {
				"error": "Invalid file (encrypted)"
			}
		}
		
		// Check if it's a spritelist first.
		let mut sprite_list: bool = true;
		let objects: Vec<Vec<u8>> = Self::get_objects(&bin_data);
		
		for object in 0..objects.len() {
			match identify_object(&objects[object]) {
				ObjectType::Sprite => continue,
				
				_ => {
					sprite_list = false;
					break;
				},
			}
		}
		
		if sprite_list {
			godot_print!("Loading SpriteListFile");
			return Self::load_sprite_list_file(bin_data);
		}
		
		else {
			return Self::load_resource_file(bin_data);
		}
	}
	
	
	// =================================================================================
	// LOADING
	// =================================================================================
	
	
	fn get_objects(bin_data: &Vec<u8>) -> Vec<Vec<u8>> {
		let header_pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		let mut objects: Vec<Vec<u8>> = Vec::new();
		
		for pointer in 0..header_pointers.len() {
			if pointer == header_pointers.len() - 1 {
				objects.push(bin_data[header_pointers[pointer]..].to_vec());
			}
			
			else {
				objects.push(bin_data[header_pointers[pointer]..header_pointers[pointer + 1]].to_vec());
			}
		}
		
		return objects;
	}
	
	
	fn load_resource_file(bin_data: Vec<u8>) -> Dictionary {
		let objects: Vec<Vec<u8>> = Self::get_objects(&bin_data);
		let mut resource_dictionary: Dictionary = Dictionary::new();
		
		// For every sub object
		let mut object_number: usize = 0;
		for object in 0..objects.len() {
			let object_bin_data: &Vec<u8> = &objects[object];
			let mut dictionary: Dictionary;
			
			// Get and load per object type
			match identify_object(&object_bin_data) {
			
			
				ObjectType::Sprite => {
					let sprite = SpriteLoadSave::load_sprite_data(object_bin_data);
					let mut array: Array<Gd<BinSprite>> = Array::new();
					
					match sprite {
						Some(bin_sprite) => {
							array.push(&bin_sprite);
							
							dictionary = dict! {
								"type": "sprite",
								"sprites": array,
							}
						},
						
						None => {
							dictionary = dict! {
								"type": "unsupported",
								"data": PackedByteArray::from(object_bin_data.clone()),
							}
						},
					}
				}
				
				
				ObjectType::SpriteListSelect => {
					let pointers: Vec<usize> = get_pointers(object_bin_data, 0x00, false);
					let last_pointer = pointers[pointers.len() - 1];
					let mut sprites = Self::load_sprite_list(object_bin_data, 0);
					let _ = sprites.pop();
					
					let select_w = u32::from_le_bytes([
						object_bin_data[last_pointer + 0x00],
						object_bin_data[last_pointer + 0x01],
						object_bin_data[last_pointer + 0x02],
						object_bin_data[last_pointer + 0x03],
					]);
					
					let select_h = u32::from_le_bytes([
						object_bin_data[last_pointer + 0x04],
						object_bin_data[last_pointer + 0x05],
						object_bin_data[last_pointer + 0x06],
						object_bin_data[last_pointer + 0x07],
					]);
					
					let select_pixels = PackedByteArray::from(
						object_bin_data[last_pointer + 0x08..last_pointer + (select_w * select_h) as usize].to_vec());
					
					dictionary = dict! {
						"type": "sprite_list_select",
						"sprites": sprites,
						"select_width": select_w,
						"select_height": select_h,
						"select_pixels": select_pixels,
					};
				},
				
				
				ObjectType::SpriteList => {
					let sprites = Self::load_sprite_list(object_bin_data, 0);
					
					dictionary = dict! {
						"type": "sprite_list",
						"sprites": sprites,
					};
				},
				
				
				ObjectType::JPFPlainText => {
					let pointers: Vec<usize> = get_pointers(object_bin_data, 0x00, false);
					
					let char_index = PackedByteArray::from(
						object_bin_data[pointers[0]..pointers[1]].to_vec()
					);
					
					let sprites: Array<Gd<BinSprite>> = Self::load_sprite_list(object_bin_data, 1);
					
					dictionary = dict! {
						"type": "jpf_plain_text",
						"char_index": char_index,
						"sprites": sprites,
					}
				},
				
				
				ObjectType::Scriptable => {
					let scriptable: Scriptable = Self::load_scriptable(object_bin_data, object_number);
					
					if scriptable.name != "Player" {
						object_number += 1;
					}
					
					dictionary = dict! {
						"type": "scriptable",
						"name": scriptable.name,
						"cells": scriptable.cells,
						"sprites": scriptable.sprites,
						"scripts": scriptable.scripts,
					};
					
					println!("Built scriptable dictionary");
					
					if scriptable.palettes.len() > 0 {
						dictionary.set("palettes", scriptable.palettes);
					}
				},
				
				
				// Only used by archive_jpf.bin, for speed,
				// assume rather than try to ID each object
				ObjectType::MultiScriptable => {
					let mut multi_scriptable: Dictionary = dict! {};
					let scriptables: Vec<Vec<u8>> = Self::get_objects(object_bin_data);
					
					for item in 0..scriptables.len() {
						let scriptable: Scriptable = Self::load_scriptable(&scriptables[item], item);
						let scriptable_dict: Dictionary = dict! {
							"name": "Effect",
							"type": "scriptable",
							"cells": scriptable.cells,
							"sprites": scriptable.sprites,
							"scripts": scriptable.scripts,
						};
						
						multi_scriptable.set(item as i64, scriptable_dict);
					}
				
					dictionary = dict! {
						"type": "multi_scriptable",
						"data": multi_scriptable,
					};
				},
				
				
				_ => {
					dictionary = dict! {
						"type": "unsupported",
						"data": PackedByteArray::from(object_bin_data.clone()),
					};
				},
			}
			
			resource_dictionary.set(object as u32, dictionary);
		}
		
		return resource_dictionary;
	}
	
	
	fn load_sprite_list_file(bin_data: Vec<u8>) -> Dictionary {
		let header_pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		let mut sprites: Array<Gd<BinSprite>> = Array::new();
		
		for sprite in 0..header_pointers.len() {
			let sprite_bin_data: Vec<u8>;
			
			if sprite == header_pointers.len() - 1 {
				sprite_bin_data = bin_data[header_pointers[sprite]..].to_vec();
			}
			
			else {
				sprite_bin_data = bin_data[header_pointers[sprite]..header_pointers[sprite + 1]].to_vec();
			}
			
			match SpriteLoadSave::load_sprite_data(&sprite_bin_data) {
				Some(bin_sprite) => sprites.push(&bin_sprite),
				_ => sprites.push(&BinSprite::new_gd()),
			}
		}
		
		return dict! {
			1u32: dict! {
				"type": "sprite_list_file",
				"sprites": sprites,
			}
		}
	}
	
	
	// =================================================================================
	// OBJECT LOADING
	// =================================================================================
	
	
	fn load_scriptable(bin_data: &Vec<u8>, number: usize) -> Scriptable {
		let pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		
		let mut name = format!("Object #{}", number);
		let cells = Self::load_cells(bin_data, &pointers);
		let sprites = Self::load_sprites(bin_data, &pointers);
		let scripts = PackedByteArray::from(Self::load_scripts(bin_data, &pointers));
		let palettes = Self::load_palettes(bin_data, &pointers);
		
		if palettes.len() > 0 {
			name = "Player".into();
		}
		
		return Scriptable {
			name: name.into(),
			cells: cells,
			sprites: sprites,
			scripts: scripts,
			palettes: palettes,
		};
	}
	
	
	fn load_cells(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Array<Gd<Cell>> {
		// Load cells
		let cell_pointers: Vec<usize> = get_pointers(&bin_data, pointers[0], false);
		let mut cells: Array<Gd<Cell>> = Array::new();

		for cell in cell_pointers.iter() {
			let cursor: usize = pointers[0] + cell;
			let hitbox_count: u32 = u32::from_le_bytes([
				bin_data[cursor + 0x00],
				bin_data[cursor + 0x01],
				bin_data[cursor + 0x02],
				bin_data[cursor + 0x03]
			]);
			
			let cell_slice: &[u8] = &bin_data[cursor..cursor + 0x10 + (hitbox_count as usize * 0x0C)];
			match Cell::from_binary_data(cell_slice) {
				Some(cell) => cells.push(&cell),
				_ => cells.push(&Cell::new_gd()),
			}
		}
		
		return cells;
	}
	
	
	fn load_sprites(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Array<Gd<BinSprite>> {
		// Load sprites
		let sprite_pointers: Vec<usize> = get_pointers(&bin_data, pointers[1], false);
		let mut sprites: Array<Gd<BinSprite>> = Array::new();
		
		for sprite in 0..sprite_pointers.len() {
			let start: usize = pointers[1] + sprite_pointers[sprite];
			let end: usize;
			
			if sprite < sprite_pointers.len() - 1 {
				end = pointers[1] + sprite_pointers[sprite + 1];
			}
			
			else {
				end = pointers[2];
			}
			
			match SpriteLoadSave::load_sprite_data(&bin_data[start..end].to_vec()) {
				Some(sprite) => {
					sprites.push(&sprite);
				},
				
				None => {
					sprites.push(&BinSprite::new_gd());
				},
			}
		}
		
		return sprites;
	}
	
	
	fn load_scripts(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Vec<u8> {
		// Load script
		let scripts: Vec<u8>;
		if pointers.len() == 4 {
			scripts = bin_data[pointers[2]..pointers[3]].to_vec();
		}
		
		else {
			scripts = bin_data[pointers[2]..].to_vec();
		}
		
		return scripts;
	}
	
	
	fn load_palettes(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Array<Gd<BinPalette>> {
		let mut palettes: Array<Gd<BinPalette>> = Array::new();
		
		// Load palettes
		if pointers.len() < 4 {
			return palettes;
		}
		
		let palette_pointers: Vec<usize> = get_pointers(&bin_data, pointers[3], false);
		for palette in palette_pointers.iter() {
			let cursor: usize = pointers[3] + palette;
			let palette_data: Vec<u8> = bin_data[cursor..cursor + 0x410].to_vec();
			
			match BinPalette::from_bin_data(palette_data) {
				Some(palette) => palettes.push(&palette),
				None => palettes.push(&BinPalette::new_gd()),
			}
		}
		
		return palettes;
	}
	
	
	fn load_sprite_list(bin_data: &Vec<u8>, from: usize) -> Array<Gd<BinSprite>> {
		let sprite_pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		let mut sprites: Array<Gd<BinSprite>> = Array::new();
		
		for sprite in from..sprite_pointers.len() {
			let sprite_data: Vec<u8>;
			let start: usize = sprite_pointers[sprite];
			
			if sprite < sprite_pointers.len() - 1 {
				let end = sprite_pointers[sprite + 1];
				sprite_data = bin_data[start..end].to_vec();
			}
			
			else {
				sprite_data = bin_data[start..].to_vec();
			}
			
			match SpriteLoadSave::load_sprite_data(&sprite_data) {
				Some(sprite) => {
					sprites.push(&sprite);
				},
				
				None => {
					sprites.push(&BinSprite::new_gd());
				}
			}
		}
		
		return sprites;
	}


	// =================================================================================
	// SAVING
	// =================================================================================
	
	
	#[func] pub fn save_resource_file(dictionary: Dictionary, path: String) {	
		let mut path_buf: PathBuf = PathBuf::from(path);
		
		if !path_buf.exists() {
			godot_print!("Path does not exist!");
			return;
		}
		
		let mut file_vector: Vec<u8> = Vec::new();
		let mut data_vector: Vec<u8> = Vec::new();
		let mut header_pointers: Vec<u32> = Vec::new();
		
		for (_object_number, object_dict) in dictionary.iter_shared().typed::<i64, Dictionary>() {
			header_pointers.push(data_vector.len() as u32);
			
			let this_type: String = object_dict.get("type").unwrap().to_string();
			
			match &this_type as &str {
				"sprite_list_file" => {
					let sprite_array: Array<Gd<BinSprite>> = object_dict.at("sprites").to();
					let sprite_block: (Vec<u32>, Vec<u8>) = Self::get_sprite_block(sprite_array, 0x00);
					
					header_pointers = sprite_block.0;
					data_vector.extend(sprite_block.1);
					break;
				}
				
				"sprite" =>
					data_vector.extend(Self::get_bin_sprite(object_dict)),
					
				"sprite_list" =>
					data_vector.extend(Self::get_bin_sprite_list(object_dict)),
					
				"sprite_list_select" =>
					data_vector.extend(Self::get_bin_sprite_list_select(object_dict)),
					
				"jpf_plain_text" =>
					data_vector.extend(Self::get_bin_jpf_plain_text(object_dict)),
					
				"scriptable" =>
					data_vector.extend(Self::get_bin_scriptable(object_dict)),
					
				"multi_scriptable" =>
					data_vector.extend(Self::get_bin_multi_scriptable(object_dict)),
					
				_ => {
					let data_array: PackedByteArray = object_dict.at("data").to();
					data_vector.extend(data_array.to_vec());
				}
			}
		}
		
		file_vector.extend(Self::finalize_pointers(header_pointers));
		file_vector.extend(data_vector);
		
		path_buf.push("test.bin");
		
		match fs::File::create(path_buf) {
			Ok(file) => {
				let ref mut buffer = BufWriter::new(file);
				let _ = buffer.write_all(&file_vector);
				let _ = buffer.flush();
			},
			
			_ => (),
		}
	}
	
	
	// GENERIC/UTILITY =================================================================
	
	
	// Separate; sometimes we need to add extra stuff before this step
	fn finalize_pointers(source_pointers: Vec<u32>) -> Vec<u8> {
		let mut target_pointers: Vec<u32> = source_pointers.clone();
		let mut target_vector: Vec<u8> = Vec::new();
		
		// Terminate and align
		let pointer_count: usize = source_pointers.len();
		target_pointers.push(0xFFFFFFFF);
		
		while target_pointers.len() % 4 != 0 {
			target_pointers.push(0xFFFFFFFF);
		}
		
		// Adjust addresses
		for pointer in 0..pointer_count {
			target_pointers[pointer] += 4 * target_pointers.len() as u32;
		}
		
		// Register as data
		for pointer in 0..target_pointers.len() {
			target_vector.extend(target_pointers[pointer].to_le_bytes());
		}
		
		return target_vector;
	}
	
	
	// Returns (non-finalized) pointer vector and sprite vector
	fn get_sprite_block(sprite_array: Array<Gd<BinSprite>>, offset: u32) -> (Vec<u32>, Vec<u8>) {
		let mut pointer_vector: Vec<u32> = Vec::new();
		let mut sprite_vector: Vec<u8> = Vec::new();
		
		for item in 0..sprite_array.len() {
			pointer_vector.push(sprite_vector.len() as u32 + offset);
			let sprite: Gd<BinSprite> = sprite_array.at(item);
			sprite_vector.extend(sprite.bind().to_bin());
		}
		
		return (pointer_vector, sprite_vector);
	}
	
	
	fn get_cell_block(cell_array: Array<Gd<Cell>>) -> (Vec<u32>, Vec<u8>) {
		let mut pointer_vector: Vec<u32> = Vec::new();
		let mut cell_vector: Vec<u8> = Vec::new();
		
		for item in 0..cell_array.len() {
			pointer_vector.push(cell_vector.len() as u32);
			let cell: Gd<Cell> = cell_array.at(item);
			cell_vector.extend(cell.bind().to_bin());
		}
		
		return (pointer_vector, cell_vector);
	}
	
	
	fn get_palette_block(palette_array: Array<Gd<BinPalette>>) -> (Vec<u32>, Vec<u8>) {
		let mut pointer_vector: Vec<u32> = Vec::new();
		let mut palette_vector: Vec<u8> = Vec::new();
		
		for item in 0..palette_array.len() {
			pointer_vector.push(palette_vector.len() as u32);
			let palette: Gd<BinPalette> = palette_array.at(item);
			palette_vector.extend(palette.bind().to_bin());
		}
		
		return (pointer_vector, palette_vector);
	}
	
	
	// OBJECT TYPES ====================================================================
	
	
	fn get_bin_sprite(dictionary: Dictionary) -> Vec<u8> {
		let sprite_array: Array<Gd<BinSprite>> = dictionary.at("sprites").to();
		let sprite: Gd<BinSprite> = sprite_array.at(0);
		return sprite.bind().to_bin();
	}
	
	
	fn get_bin_sprite_list(dictionary: Dictionary) -> Vec<u8> {
		let sprite_array: Array<Gd<BinSprite>> = dictionary.at("sprites").to();
		
		let header_pointers: Vec<u32>;
		let data_vector: Vec<u8>;
		(header_pointers, data_vector) = Self::get_sprite_block(sprite_array, 0x00);
		
		// Write
		let mut object_vector: Vec<u8> = Self::finalize_pointers(header_pointers);
		object_vector.extend(data_vector);
		
		return object_vector;
	}
	
	
	fn get_bin_sprite_list_select(dictionary: Dictionary) -> Vec<u8> {
		/* Dictionary contents:
		 * "type": "sprite_list_select"
		 * "sprites": Array<Gd<BinSprite>>
		 * "select_width": i64
		 * "select_height": i64,
		 * "select_pixels": PackedByteArray,
		 */
		
		/* Data format:
		 * 178 u32 pointers, last is select
		 * select:
		 * 		u32 width
		 *		u32 height
		 *		Vec<u8> raw pixel array
		 */
		
		let sprite_array: Array<Gd<BinSprite>> = dictionary.at("sprites").to();
		
		let mut header_pointers: Vec<u32>;
		let mut data_vector: Vec<u8>;
		(header_pointers, data_vector) = Self::get_sprite_block(sprite_array, 0x00);
		
		// Add select pointer
		header_pointers.push(data_vector.len() as u32);
		
		let select_width: u32 = dictionary.at("select_width").to();
		let select_height: u32 = dictionary.at("select_height").to();
		let select_pixels: PackedByteArray = dictionary.at("select_pixels").to();
		
		data_vector.extend(select_width.to_le_bytes());
		data_vector.extend(select_height.to_le_bytes());
		data_vector.extend(select_pixels.to_vec());
		
		while data_vector.len() % 0x10 != 0 {
			data_vector.push(0xFF);
		}
		
		let mut object_vector: Vec<u8> = Self::finalize_pointers(header_pointers);
		object_vector.extend(data_vector);
		
		return object_vector;
	}


	fn get_bin_jpf_plain_text(dictionary: Dictionary) -> Vec<u8> {
		/* Dictionary contents:
		 * "type": "jpf_plain_text",
		 * "char_index": PackedByteArray,
		 * "sprites": Array<Gd<BinSprite>>,
		 */
		
		/* Data format:
		 * u32 pointers, first is char_index, then sprites
		 * char_index:
		 *		PackedByteArray -> passthru
		 */
		 
		let char_index: PackedByteArray = dictionary.at("char_index").to();
		
		let mut header_pointers: Vec<u32> = vec![0u32];
		let mut data_vector: Vec<u8>;
		
		data_vector = char_index.to_vec();
		
		// Ensure terminated, just in case
		// Shouldn't come into play with passthru of well-formed data
		while data_vector.len() % 0x10 != 0 {
			data_vector.push(0xFF);
		}
		
		let sprite_array: Array<Gd<BinSprite>> = dictionary.at("sprites").to();
		let sprite_block = Self::get_sprite_block(sprite_array, data_vector.len() as u32);
		
		header_pointers.extend(sprite_block.0);
		data_vector.extend(sprite_block.1);
		
		let mut object_vector: Vec<u8> = Self::finalize_pointers(header_pointers);
		object_vector.extend(data_vector);
		
		return object_vector;
	}
	
	
	fn get_bin_scriptable(dictionary: Dictionary) -> Vec<u8> {
		/* Dictionary contents:
		 * "type": "scriptable",
		 * "name": String,
		 * "cells": Array<Gd<Cell>>,
		 * "sprites": Array<Gd<BinSprite>>,
		 * "scripts": PackedByteArray,
		 * "palettes": Array<Gd<BinPalette>>,
		 */
		
		/* Data format:
		 * u32 pointers,
		 * cells,
		 * sprites,
		 * scripts,
		 * palettes
		 */
		
		let mut header_pointers: Vec<u32> = Vec::new();
		let mut data_vector: Vec<u8> = Vec::new();
		
		// Cells
		let cell_array: Array<Gd<Cell>> = dictionary.at("cells").to();
		let cell_tuple: (Vec<u32>, Vec<u8>) = Self::get_cell_block(cell_array);
		let cell_pointers: Vec<u8> = Self::finalize_pointers(cell_tuple.0);
		
		// Sprites
		let sprite_array: Array<Gd<BinSprite>> = dictionary.at("sprites").to();
		let sprite_tuple: (Vec<u32>, Vec<u8>) = Self::get_sprite_block(sprite_array, 0x00);
		let sprite_pointers: Vec<u8> = Self::finalize_pointers(sprite_tuple.0);
		
		// Scripts
		let scripts_array: PackedByteArray = dictionary.at("scripts").to();
		let scripts: Vec<u8> = scripts_array.to_vec();
		
		// Start writing...
		header_pointers.push(data_vector.len() as u32);
		data_vector.extend(cell_pointers);
		data_vector.extend(cell_tuple.1);
		
		header_pointers.push(data_vector.len() as u32);
		data_vector.extend(sprite_pointers);
		data_vector.extend(sprite_tuple.1);
		
		header_pointers.push(data_vector.len() as u32);
		data_vector.extend(scripts);
		
		match dictionary.get("palettes") {
			Some(value) => {
				let palette_array: Array<Gd<BinPalette>> = value.to();
				let palette_tuple: (Vec<u32>, Vec<u8>) = Self::get_palette_block(palette_array);
				let palette_pointers: Vec<u8> = Self::finalize_pointers(palette_tuple.0);
				
				header_pointers.push(data_vector.len() as u32);
				data_vector.extend(palette_pointers);
				data_vector.extend(palette_tuple.1);
			}
			
			_ => (),
		}
		
		let mut object_vector: Vec<u8> = Self::finalize_pointers(header_pointers);
		object_vector.extend(data_vector);
		
		return object_vector;
	}
	
	
	fn get_bin_multi_scriptable(dictionary: Dictionary) -> Vec<u8> {
		/* Dictionary contents:
		 * "type": "multi_scriptable",
		 * "data": Dictionary {
		 *		0: Dictionary {
		 *			"name": "Whatever",
		 *			"type": "scriptable",
		 *			"cells": Array<Gd<Cell>>,
		 *			"sprites": Array<Gd<BinSprite>>,
		 *			"scripts": PackedByteArray,
		 *		}
		 *		...
		 * }
		 */
		
		let mut header_pointers: Vec<u32> = Vec::new();
		let mut data_vector: Vec<u8> = Vec::new();
		
		let inner_dict: Dictionary = dictionary.at("data").to();
		
		for item in 0..inner_dict.len() {
			header_pointers.push(data_vector.len() as u32);
			data_vector.extend(Self::get_bin_scriptable(inner_dict.at(item as i64).to()));
		}
		
		let mut object_vector: Vec<u8> = Self::finalize_pointers(header_pointers);
		object_vector.extend(data_vector);
		
		return object_vector;
	}
}