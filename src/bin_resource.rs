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
use crate::object_data::ObjectData;

/* file_type:
 * 		"multi_object" 	<-	Let's start with character binary support first.
 *							How do you identify it?
 *							1a. Header with variable object count.
 *							1b. Last object may be audio array (BE pointers, VAGp or DNBW sounds)
 *							2. First object has exactly 4 pointers.
 *							3. All other objects have exactly 3 pointers.
 *		"archive_jpf"
 */

/* object_type:
 *		"scriptable"			<- cells, sprites, script.
 *		"sprite_list"			<- pointers to individual sprites.
 *		"sprite_list_select"	<- pointers to individual sprites, then the select screen bitmask.
 *		"arcjpf_plain_text"		<- char_index.bin, then individual sprites.
 *		"wii_tpl"				<- Wii TPL texture.
 *		"multi_object"			<- contains scriptable subobjects (archive_jpf.bin effects)
 *		"dummy"					<- "DUMMY" padding.
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
	#[func]
	fn from_file(source_path: String) -> Dictionary {
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
		
		return Self::load_resource_file(bin_data);
	}
	
	
	// =================================================================================
	// LOADING
	// =================================================================================
	
	
	fn load_resource_file(bin_data: Vec<u8>) -> Dictionary {
		let header_pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		let mut resource_dictionary: Dictionary = Dictionary::new();
		
		// For every sub object
		let mut object_number: usize = 0;
		for object in 0..header_pointers.len() {
			let object_bin_data: Vec<u8>;
			let mut dictionary: Dictionary;
			
			// If this is the last object
			if object == header_pointers.len() - 1 {
				object_bin_data = bin_data[header_pointers[object]..].to_vec();
			}
			
			else {
				object_bin_data = bin_data[header_pointers[object]..header_pointers[object + 1]].to_vec();
			}
			
			// Get and load per object type
			match identify_object(&object_bin_data) {
			
			
				ObjectType::Sprite => {
					let sprite = SpriteLoadSave::load_sprite_data(&object_bin_data);
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
								"data": PackedByteArray::from(object_bin_data),
							}
						},
					}
				}
				
				
				ObjectType::SpriteListSelect => {
					let pointers: Vec<usize> = get_pointers(&object_bin_data, 0x00, false);
					let last_pointer = pointers[pointers.len() - 1];
					let mut sprites = Self::load_sprite_list(&object_bin_data, 0);
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
					let sprites = Self::load_sprite_list(&object_bin_data, 0);
					
					dictionary = dict! {
						"type": "sprite_list",
						"sprites": sprites,
					};
				},
				
				
				ObjectType::JPFPlainText => {
					let pointers: Vec<usize> = get_pointers(&object_bin_data, 0x00, false);
					
					let char_index = PackedByteArray::from(
						object_bin_data[pointers[0]..pointers[1]].to_vec()
					);
					
					let sprites: Array<Gd<BinSprite>> = Self::load_sprite_list(&object_bin_data, 1);
					
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
					
					if scriptable.palettes.len() > 0 {
						dictionary.set("palettes", scriptable.palettes);
					}
				},
				
				
				// Only used by archive_jpf.bin, for speed,
				// assume rather than try to ID each object
				ObjectType::MultiScriptable => {
					let scriptable_pointers: Vec<usize> = get_pointers(&object_bin_data, 0x00, false);
					let mut scriptable_list: Dictionary = dict! {};
					
					for pointer in 0..scriptable_pointers.len() {
						let scriptable_bin_data: Vec<u8>;
						let start: usize = scriptable_pointers[pointer];
						
						if pointer < scriptable_pointers.len() - 1 {
							let end: usize = scriptable_pointers[pointer + 1];
							scriptable_bin_data = object_bin_data[start..end].to_vec();
						}
						
						else {
							scriptable_bin_data = object_bin_data[start..].to_vec();
						}
						
						let scriptable: Scriptable = Self::load_scriptable(scriptable_bin_data, pointer);
						let scriptable_dict: Dictionary = dict! {
							"name": format!("Effect #{}", pointer),
							"type": "scriptable",
							"cells": scriptable.cells,
							"sprites": scriptable.sprites,
							"scripts": scriptable.scripts,
						};
						
						scriptable_list.set(pointer as i64, scriptable_dict);
					}
				
					dictionary = dict! {
						"type": "multi_scriptable",
						"data": scriptable_list,
					};
				},
				
				
				_ => {
					dictionary = dict! {
						"type": "unsupported",
						"data": PackedByteArray::from(object_bin_data),
					};
				},
			}
			
			resource_dictionary.set(object as u32, dictionary);
		}
		
		return resource_dictionary;
	}
	
	
	// =================================================================================
	// OBJECT LOADING
	// =================================================================================
	
	
	fn load_scriptable(bin_data: Vec<u8>, number: usize) -> Scriptable {
		let pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
		
		let mut name = format!("Object #{}", number);
		let cells = Self::load_cells(&bin_data, &pointers);
		let sprites = Self::load_sprites(&bin_data, &pointers);
		let scripts = PackedByteArray::from(Self::load_scripts(&bin_data, &pointers));
		let palettes = Self::load_palettes(&bin_data, &pointers);
		
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
		
		for (_object, data) in dictionary.iter_shared() {
			let this_dict: Dictionary = data.to();
							
			header_pointers.push(data_vector.len() as u32);
			
			match this_dict.get("type") {
				Some(string_type) => {
					// Comes in as a Variant
					let string: String = string_type.to();
					
					match string.as_str() {
						"object" => {
							let object_data: Gd<ObjectData> = this_dict.at("data").to();
							let binding = object_data.bind();
							let binary_data: Vec<u8> = binding.get_as_binary();
							data_vector.extend(binary_data);
						},
						
						_ => {
							let packed_array: PackedByteArray = this_dict.at("data").to();
							data_vector.extend(packed_array.to_vec());
						},
					}
				},
				
				_ => godot_print!("Invalid dictionary entry!"),
			}
		}
		
		// Pad pointers
		let pointer_count: usize = header_pointers.len();
		
		header_pointers.push(0xFFFFFFFF);
		
		while header_pointers.len() % 4 != 0 {
			header_pointers.push(0xFFFFFFFF);
		}
		
		for pointer in 0..pointer_count {
			header_pointers[pointer] += 4 * header_pointers.len() as u32;
		}
		
		for pointer in 0..header_pointers.len() {
			file_vector.extend(header_pointers[pointer].to_le_bytes());
		}
		
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
}