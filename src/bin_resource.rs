use std::fs;
use std::path::PathBuf;

use godot::prelude::*;

use crate::bin_sprite::BinSprite;
use crate::bin_cell::Cell;
use crate::bin_palette;
use crate::bin_palette::BinPalette;
use crate::sprite_load_save::SpriteLoadSave;


enum ResourceType {
	Character,
	ArchiveJPF,
	SpriteList,
	SpriteObjects,
	Select,
}


enum ResourceError {
	TooShort,
	Encrypted,
	NoPointers,
	Unidentified,
}


const WBND_SIGNATURE: usize = 0x444E4257;
const VAGP_SIGNATURE: [u8; 4] = [0x56, 0x41, 0x47, 0x70];


const SPRITE_SIGNATURES: [[u8; 6]; 8] = [
	// Uncompressed
	[0x00, 0x00, 0x00, 0x00, 0x04, 0x00],	// No palette, 4bpp
	[0x00, 0x00, 0x00, 0x00, 0x08, 0x00],	// No palette, 8bpp
	[0x00, 0x00, 0x20, 0x00, 0x04, 0x00],	// Palette, 4bpp
	[0x00, 0x00, 0x20, 0x00, 0x08, 0x00],	// Palette, 8bpp
	// Compressed
	[0x01, 0x00, 0x00, 0x00, 0x04, 0x00],	// No palette, 4bpp
	[0x01, 0x00, 0x00, 0x00, 0x08, 0x00],	// No palette, 8bpp
	[0x01, 0x00, 0x20, 0x00, 0x04, 0x00],	// Palette, 4bpp
	[0x01, 0x00, 0x20, 0x00, 0x08, 0x00],	// Palette, 8bpp
];

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


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R binary resource file loader.
struct ResourceLoadSave {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for ResourceLoadSave {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl ResourceLoadSave {
	/// Loads a BIN resource file, returning the objects contained within.
	#[func]
	fn load_file(source_path: GString) -> Dictionary {
		let path_str: String = String::from(source_path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			return dict! {
				"error": "File not found!",
			}
		}
		
		match fs::read(path_buf) {
			Ok(data) => {
				return Self::load_binary_data(data);
			},
			
			_ => return dict! {
				"error": "Could not load binary file!",
			},
		}
	}


	fn load_binary_data(bin_data: Vec<u8>) -> Dictionary {
		match Self::identify_data(&bin_data) {
			Ok(data_type) => match data_type {
				ResourceType::Character => return Self::load_character(bin_data),
				// ResourceType.ArchiveJPF => return load_archive_jpf(bin_data),
				// ResourceType.SpriteList => return load_sprite_list(bin_data),
				// ResourceType.SpriteObjects => return load_sprite_objects(bin_data),
				// ResourceType.Select => return load_select(bin_data),
				_ => return dict! {
						"error": "Unsupported binary file type.",
				},
			},
			
			Err(err_type) => match err_type {
				ResourceError::TooShort => return dict! {
					"error": "File too short!",
				},
				ResourceError::Encrypted => return dict! {
					"error": "File encrypted!",
				},
				ResourceError::NoPointers => return dict! {
					"error": "File has no pointers!",
				},
				ResourceError::Unidentified => return dict! {
					"error": "Unidentified file type!",
				},
			}
		}
	}
	
	
	fn identify_data(bin_data: &Vec<u8>) -> Result<ResourceType, ResourceError> {
		let bin_len: usize = bin_data.len();
		
		if bin_len < 0x22 {
			return Err(ResourceError::TooShort);
		}
		
		if &bin_data[bin_len - 0x04..] == &[0x41, 0x53, 0x47, 0x43] {
			return Err(ResourceError::Encrypted);
		}
		
		// Get header pointers to sub objects
		let header_pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		
		let mut cursor: usize = header_pointers.len() * 0x04;
		cursor = (cursor / 0x10) * 0x10 + 0x10;
		
		if header_pointers.len() == 0 {
			return Err(ResourceError::NoPointers);
		}
		
		// Identify SpriteList
		if true {
			let spr_sig_check: &[u8; 6] = &[
				bin_data[cursor + 0x00], bin_data[cursor + 0x01],
				bin_data[cursor + 0x02], bin_data[cursor + 0x03],
				bin_data[cursor + 0x04], bin_data[cursor + 0x05],
			];
			
			if SPRITE_SIGNATURES.contains(spr_sig_check) {
				return Ok(ResourceType::SpriteList);
			}
		}
		
		// Identify Character
		if true {
			let mut positive_id: bool = true;
			
			for pointer in 0..header_pointers.len() {
				let pointers_this_object: Vec<usize> = Self::get_pointers(
					&bin_data, header_pointers[pointer], false);
				
				// Check if first object contains exactly four pointers, ...
				if pointer == 0 {
					if pointers_this_object.len() != 4 {
						positive_id = false;
						break;
					}
				}
				
				// ... then if other objects contain exactly three pointers.
				else {
					if pointers_this_object.len() != 3 {
						// OK to have != 3 pointers if it's an audio array
						let audio_array_pointer: usize = (pointers_this_object[0] as u32).swap_bytes() as usize;
						
						if audio_array_pointer == WBND_SIGNATURE {
							continue;
						}
						
						let signature: Vec<u8> = vec![
							bin_data[header_pointers[pointer] + audio_array_pointer + 0x00],
							bin_data[header_pointers[pointer] + audio_array_pointer + 0x01],
							bin_data[header_pointers[pointer] + audio_array_pointer + 0x02],
							bin_data[header_pointers[pointer] + audio_array_pointer + 0x03],
						];
						
						if signature == VAGP_SIGNATURE {
							continue;
						}
						
						positive_id = false;
						break;
					}
				}
			}
			
			if positive_id {
				return Ok(ResourceType::Character);
			}
		}
		
		return Err(ResourceError::Unidentified);
	}
	
	
	fn get_pointers(bin_data: &Vec<u8>, mut cursor: usize, big_endian: bool) -> Vec<usize> {
		let mut pointers: Vec<usize> = Vec::new();
		
		loop {
			if cursor + 3 >= bin_data.len() {
				break;
			}
			
			let pointer: usize;
			
			if big_endian {
				pointer = u32::from_be_bytes(
					[
						bin_data[cursor + 0x00],
						bin_data[cursor + 0x01],
						bin_data[cursor + 0x02],
						bin_data[cursor + 0x03]
					]
				) as usize;
			}
			
			else {
				pointer = u32::from_le_bytes(
					[
						bin_data[cursor + 0x00],
						bin_data[cursor + 0x01],
						bin_data[cursor + 0x02],
						bin_data[cursor + 0x03]
					]
				) as usize;
			}
			
			if pointer == 0xFFFFFFFF {
				break;
			}
			
			pointers.push(pointer);
			cursor += 4;
		}
		
		return pointers;
	}
	
	
	fn load_character(bin_data: Vec<u8>) -> Dictionary {
		let header_pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		let mut character_dictionary: Dictionary = Dictionary::new();
		
		// Let's start by only loading the 'player' object.
		
		/*	"objects": {
				"player": {
					"cells": Vec<BinCell>,
					"sprites": Vec<BinSprite>,
					"script": PackedByteArray,
					"palettes": Vec<BinPalette>,
				},
				
				"objno0": {
					"cells": Vec<PackedByteArray>,
					"sprites": Vec<BinSprite>,
					"script": PackedByteArray,
				},
				
				etc ...
			}
		*/
		
		for object in 0..header_pointers.len() {
			let new_object: Dictionary;
		
			if object == header_pointers.len() - 1 {
				new_object = Self::load_character_object(
					&bin_data[header_pointers[object]..].to_vec()
				);
			}
			
			else {
				new_object = Self::load_character_object(
					&bin_data[header_pointers[object]..header_pointers[object + 1]].to_vec()
				);
			}
			
			character_dictionary.set(object as i32, new_object);
		}
		
		return character_dictionary;
	}
	
	
	fn load_character_object(bin_data: &Vec<u8>) -> Dictionary {
		let pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		let pointers_vagp: Vec<usize> = Self::get_pointers(&bin_data, 0x00, true);
		
		let mut object_dictionary: Dictionary = Dictionary::new();
		
		// Check if WBND first
		if pointers_vagp[0] == WBND_SIGNATURE {
			return dict! {
				"type": "audio_wbnd",
				"data": PackedByteArray::from(bin_data.clone()),
			}
		}
		
		// Check if VAGp first
		if bin_data.len() > pointers_vagp[0] {
			let signature: Vec<u8> = vec![
				bin_data[pointers_vagp[0] + 0x00],
				bin_data[pointers_vagp[0] + 0x01],
				bin_data[pointers_vagp[0] + 0x02],
				bin_data[pointers_vagp[0] + 0x03]
			];
			
			if signature == VAGP_SIGNATURE {
				return dict! {
					"type": "audio_vagp",
					"data": PackedByteArray::from(bin_data.clone()),
				};
			}
		}
		
		object_dictionary.set("cells", Self::load_cells(&bin_data, &pointers));
		object_dictionary.set("sprites", Self::load_sprites(&bin_data, &pointers));
		object_dictionary.set("script", Self::load_script(&bin_data, &pointers));
		
		match Self::load_palettes(&bin_data, &pointers) {
			Some(palette_array) => {
				object_dictionary.set("palettes", palette_array);
				object_dictionary.set("type", "player");
			},
			
			None => object_dictionary.set("type", "object"),
		}
		
		return object_dictionary;
	}
	
	
	fn load_cells(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Vec<Gd<Cell>> {
		// Load cells
		let cell_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[0], false);
		let mut cells: Vec<Gd<Cell>> = Vec::new();

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
				Some(cell) => cells.push(cell),
				_ => continue,
			}
		}
		
		return cells;
	}
	
	
	fn load_sprites(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Vec<Gd<BinSprite>> {
		// Load sprites
		let sprite_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[1], false);
		let mut sprites: Vec<Gd<BinSprite>> = Vec::new();
		
		for sprite in 0..sprite_pointers.len() {
			let start: usize = pointers[1] + sprite_pointers[sprite];
			let end: usize;
			
			if sprite < sprite_pointers.len() - 1 {
				end = pointers[1] + sprite_pointers[sprite + 1];
			}
			
			else {
				end = pointers[2];
			}
			
			match SpriteLoadSave::load_sprite_data(bin_data[start..end].to_vec()) {
				Some(sprite) => {
					sprites.push(sprite);
				},
				
				None => {
					continue;
				},
			}
		}
		
		return sprites;
	}
	
	
	fn load_script(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Vec<u8> {
		// Load script
		let script: Vec<u8>;
		if pointers.len() == 4 {
			script = bin_data[pointers[2]..pointers[3]].to_vec();
		}
		
		else {
			script = bin_data[pointers[2]..].to_vec();
		}
		
		return script;
	}
	
	
	fn load_palettes(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Option<Vec<Gd<BinPalette>>> {
		// Load palettes
		if pointers.len() < 4 {
			return None;
		}
		
		let palette_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[3], false);
		let mut palettes: Vec<Gd<BinPalette>> = Vec::new();
		
		for palette in palette_pointers.iter() {
			let cursor: usize = pointers[3] + palette;
			let palette_data: Vec<u8> = bin_data[cursor..cursor + 0x410].to_vec();
			
			match bin_palette::from_bin_data(palette_data) {
				Some(palette) => palettes.push(palette),
				None => continue,
			}
		}
		
		return Some(palettes);
	}
}