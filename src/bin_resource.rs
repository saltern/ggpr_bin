use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::path::PathBuf;

use godot::prelude::*;

use crate::bin_sprite::BinSprite;
use crate::bin_cell::Cell;
use crate::bin_palette::BinPalette;
use crate::sprite_load_save::SpriteLoadSave;
use crate::object_data::ObjectData;


const ENCRYPTED_SIGNATURE: u32 = 0x41534743;
const CHARIDX_SIGNATURE: u32 = 0x082A2000;
const WII_TPL_SIGNATURE: u32 = 0x0020AF30;
const WBND_SIGNATURE: usize = 0x444E4257;
const VAGP_SIGNATURE: u32 = 0x56414770;
const PALETTE_SIGNATURE: u32 = 0x03002000;

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

const CELL_LEN_BASE: usize = 0x10;
const CELL_LEN_BOX: usize = 0xC;

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


#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
pub enum ResourceType {
	Undefined,
	Character,		// Can have audio arrays
	ArchiveJPF,
	SpriteList,
	SpriteObjects,
	Select,
}


#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
pub enum ResourceError {
	TooShort,
	Encrypted,
	Unidentified,
}


#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
pub enum ObjectType {
	Scriptable,			// Good
	MultiScriptable,
	Sprite,
	SpriteList,			// Good
	SpriteListSelect,	// Good
	JPFPlainText,		
	WiiTPL,				// Unnecessary?
	Dummy,
	Unsupported,
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
	// UTILITY
	// =================================================================================
	
	
	fn get_pointers(bin_data: &Vec<u8>, mut cursor: usize, big_endian: bool) -> Vec<usize> {
		let mut pointers: Vec<usize> = Vec::new();
		
		loop {
			if cursor + 3 >= bin_data.len() {
				break;
			}
			
			let pointer: usize;
			
			if big_endian {
				pointer = u32::from_be_bytes([
					bin_data[cursor + 0x00],
					bin_data[cursor + 0x01],
					bin_data[cursor + 0x02],
					bin_data[cursor + 0x03]
				]) as usize;
			}
			
			else {
				pointer = u32::from_le_bytes([
					bin_data[cursor + 0x00],
					bin_data[cursor + 0x01],
					bin_data[cursor + 0x02],
					bin_data[cursor + 0x03]
				]) as usize;
			}
			
			if pointer == 0xFFFFFFFF {
				break;
			}
			
			pointers.push(pointer);
			cursor += 4;
		}
		
		return pointers;
	}
	
	
	// =================================================================================
	// IDENTIFYING
	// =================================================================================
	
	
	fn identify_audio_wbnd(bin_data: &Vec<u8>) -> bool {
		let pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, true);
		return pointers[0] == WBND_SIGNATURE;
	}
	
	
	fn identify_audio_vagp(bin_data: &Vec<u8>) -> bool {
		let pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, true);
		if bin_data.len() <= pointers[0] {
			return false;
		}
		
		return u32::from_be_bytes([
			bin_data[pointers[0] + 0x00],
			bin_data[pointers[0] + 0x01],
			bin_data[pointers[0] + 0x02],
			bin_data[pointers[0] + 0x03],
		]) == VAGP_SIGNATURE;
	}
	
	
	fn identify_sprite_list_bitmask(bin_data: &Vec<u8>) -> bool {
		if !Self::identify_sprite_list(bin_data) {
			return false;
		}
		
		let header_pointers: Vec<usize> = Self::get_pointers(bin_data, 0x00, false);
		
		// Check bitmask
		let header_last: usize = header_pointers[header_pointers.len() - 1];
		let bitmask: Vec<u8> = bin_data[header_last..].to_vec();
		
		let bitmask_w: u32 = u32::from_le_bytes([
			bitmask[0x00], bitmask[0x01],
			bitmask[0x02], bitmask[0x03],
		]);
		
		let bitmask_h: u32 = u32::from_le_bytes([
			bitmask[0x04], bitmask[0x05],
			bitmask[0x06], bitmask[0x07],
		]);
		
		let target_len: usize = 0x08 + (bitmask_w * bitmask_h) as usize;
		return bitmask.len() == target_len + target_len % 0x10;
	}
	
	
	fn identify_sprite(bin_data: &Vec<u8>) -> bool {
		if bin_data.len() < 0x20 {
			return false;
		}
		
		let sprite_signature_check: [u8; 6] = [
			bin_data[0x00], bin_data[0x01],
			bin_data[0x02], bin_data[0x03],
			bin_data[0x04], bin_data[0x05],
		];
		
		return SPRITE_SIGNATURES.contains(&sprite_signature_check);
	}
	
	
	fn identify_sprite_list(bin_data: &Vec<u8>) -> bool {
		let header_pointers: Vec<usize> = Self::get_pointers(bin_data, 0x00, false);
		
		if header_pointers.len() < 1 {
			return false;
		}
		
		let cursor: usize = header_pointers[0];
		
		if cursor + 0x05 >= bin_data.len() {
			return false;
		}
		
		let sprite_signature_check: [u8; 6] = [
			bin_data[cursor + 0x00], bin_data[cursor + 0x01],
			bin_data[cursor + 0x02], bin_data[cursor + 0x03],
			bin_data[cursor + 0x04], bin_data[cursor + 0x05],
		];
		
		return SPRITE_SIGNATURES.contains(&sprite_signature_check);
	}
	
	
	fn identify_jpf_plain_text(bin_data: &Vec<u8>) -> bool {
		let header_pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		
		if header_pointers.len() < 2 {
			return false;
		}
		
		let cursor_char_idx = header_pointers[0];
		
		if cursor_char_idx + 0x03 >= bin_data.len() {
			return false;
		}
		
		let char_idx_check: bool = u32::from_be_bytes([
			bin_data[cursor_char_idx + 0x00],
			bin_data[cursor_char_idx + 0x01],
			bin_data[cursor_char_idx + 0x02],
			bin_data[cursor_char_idx + 0x03],
		]) == CHARIDX_SIGNATURE;
		
		let cursor_sprite = header_pointers[1];
		
		if cursor_sprite + 0x05 >= bin_data.len() {
			return false;
		}
		
		let sprite_check: bool = SPRITE_SIGNATURES.contains(&[
			bin_data[cursor_sprite + 0x00], bin_data[cursor_sprite + 0x01],
			bin_data[cursor_sprite + 0x02], bin_data[cursor_sprite + 0x03],
			bin_data[cursor_sprite + 0x04], bin_data[cursor_sprite + 0x05],
		]);
		
		return char_idx_check && sprite_check;
	}
	
	
	fn identify_wii_tpl(bin_data: &Vec<u8>) -> bool {
		return u32::from_be_bytes([
			bin_data[0x00], bin_data[0x01], bin_data[0x02], bin_data[0x03],
		]) == WII_TPL_SIGNATURE;
	}
	
	
	fn identify_scriptable(bin_data: &Vec<u8>) -> bool {
		let header_pointers: Vec<usize> = Self::get_pointers(bin_data, 0x00, false);
		
		// Needs at least 1 sprite
		if header_pointers.len() < 2 {
			return false;
		}
		
		// Cells, sprites, scripts, palettes
		if header_pointers.len() > 4 {
			return false;
		}
		
		// Speedup for player objects
		if header_pointers.len() == 4 {
			let palette_pointer: usize = header_pointers[3] + u32::from_le_bytes([
				bin_data[header_pointers[3] + 0x00],
				bin_data[header_pointers[3] + 0x01],
				bin_data[header_pointers[3] + 0x02],
				bin_data[header_pointers[3] + 0x03],
			]) as usize;
			
			let palette_signature_check: u32 = u32::from_be_bytes([
				bin_data[palette_pointer + 0x00],
				bin_data[palette_pointer + 0x01],
				bin_data[palette_pointer + 0x02],
				bin_data[palette_pointer + 0x03],
			]);
			
			if palette_signature_check != PALETTE_SIGNATURE {
				return false;
			}
		}
		
		// CELL CHECK ==============================================================
		
		let mut cursor_cell_pointers: usize = header_pointers[0];
		let mut cell_pointers: Vec<usize> = Vec::new();
		
		// Get up to 3 pointers if possible
		loop {
			if bin_data.len() <= cursor_cell_pointers + 0x03 {
				break;
			}
		
			let this_pointer: usize = u32::from_le_bytes([
				bin_data[cursor_cell_pointers + 0x00],
				bin_data[cursor_cell_pointers + 0x01],
				bin_data[cursor_cell_pointers + 0x02],
				bin_data[cursor_cell_pointers + 0x03],
			]) as usize;
			
			if this_pointer == 0xFFFFFFFF {
				break;
			}
			
			cursor_cell_pointers += 0x04;
			cell_pointers.push(this_pointer);
			
			if cell_pointers.len() > 2 {
				break;
			}
		}
		
		cursor_cell_pointers = header_pointers[0];
		
		// If there's only one pointer, there's nothing to compare to.
		// Ensure at least two.
		if cell_pointers.len() < 3 {
			cell_pointers.push(header_pointers[1]);
		}
		
		for pointer in 0..cell_pointers.len() - 1 {
			let start: usize = cursor_cell_pointers + cell_pointers[pointer];
			let end: usize = cursor_cell_pointers + cell_pointers[pointer + 1];
			let this_cell: Vec<u8> = bin_data[start..end].to_vec();
			
			let box_count: usize = u32::from_le_bytes([
				this_cell[0x00], this_cell[0x01],
				this_cell[0x02], this_cell[0x03],
			]) as usize;
			
			let mut target_len: usize = CELL_LEN_BASE + CELL_LEN_BOX * box_count;
			
			if target_len % 0x10 != 0 {
				target_len += 0x10 - (target_len % 0x10);
			}
			
			if this_cell.len() != target_len {
				return false;
			}
		}
		
		// SPRITE CHECK ============================================================
		
		let cursor_sprite_pointers: usize = header_pointers[1];
		
		if bin_data.len() <= cursor_sprite_pointers + 0x03 {
			return false;
		}
		
		let sprite_pointer: usize = cursor_sprite_pointers + u32::from_le_bytes([
			bin_data[cursor_sprite_pointers + 0x00],
			bin_data[cursor_sprite_pointers + 0x01],
			bin_data[cursor_sprite_pointers + 0x02],
			bin_data[cursor_sprite_pointers + 0x03],
		]) as usize;
		
		if bin_data.len() < sprite_pointer + 0x20 {
			return false;
		}
		
		let sprite_signature_check: [u8; 6] = [
			bin_data[sprite_pointer + 0x00], bin_data[sprite_pointer + 0x01],
			bin_data[sprite_pointer + 0x02], bin_data[sprite_pointer + 0x03],
			bin_data[sprite_pointer + 0x04], bin_data[sprite_pointer + 0x05],
		];
		
		if !SPRITE_SIGNATURES.contains(&sprite_signature_check) {
			return false;
		}
		
		return true;
	}
	
	
	fn identify_multi_scriptable(bin_data: &Vec<u8>) -> bool {
		let header_pointers: Vec<usize> = Self::get_pointers(bin_data, 0x00, false);
		
		for pointer in 0..header_pointers.len() {
			let pointers_this_object: Vec<usize> = Self::get_pointers(
				&bin_data, header_pointers[pointer], false);
			
			// As far as I can tell only archive_jpf.bin has a MultiScriptable type object,
			// and they all have exactly three pointers (cells, sprites, scripts)
			if pointers_this_object.len() != 3 {
				return false;
			}
			
			let cursor_object: usize = header_pointers[pointer];
			
			let sprite_pointer: usize = u32::from_le_bytes([
				bin_data[cursor_object + pointers_this_object[1] + 0x00],
				bin_data[cursor_object + pointers_this_object[1] + 0x01],
				bin_data[cursor_object + pointers_this_object[1] + 0x02],
				bin_data[cursor_object + pointers_this_object[1] + 0x03],
			]) as usize;
			
			let cursor_sprite: usize = cursor_object + pointers_this_object[1] + sprite_pointer;
			
			if !SPRITE_SIGNATURES.contains(&[
				bin_data[cursor_sprite + 0x00], bin_data[cursor_sprite + 0x01],
				bin_data[cursor_sprite + 0x02], bin_data[cursor_sprite + 0x03],
				bin_data[cursor_sprite + 0x04], bin_data[cursor_sprite + 0x05],
			]) {
				return false;
			}
		}
		
		return true;
	}
	
	
	fn identify_object(bin_data: &Vec<u8>) -> ObjectType {
		if Self::identify_audio_wbnd(bin_data) {
			godot_print!("Identified AudioArrayWBND");
			return ObjectType::Unsupported;
		}
		
		if Self::identify_audio_vagp(bin_data) {
			godot_print!("Identified AudioArrayVAGp");
			return ObjectType::Unsupported;
		}
		
		if Self::identify_sprite(bin_data) {
			godot_print!("Identified Sprite");
			return ObjectType::Sprite;
		}
		
		if Self::identify_sprite_list_bitmask(bin_data) {
			godot_print!("Identified SpriteListSelect");
			return ObjectType::SpriteListSelect;
		}
		
		if Self::identify_sprite_list(bin_data) {
			godot_print!("Identified SpriteList");
			return ObjectType::SpriteList;
		}
		
		if Self::identify_jpf_plain_text(bin_data) {
			godot_print!("Identified JPFPlainText");
			return ObjectType::JPFPlainText;
		}
		
		if Self::identify_wii_tpl(bin_data) {
			godot_print!("Identified WiiTPL");
			return ObjectType::WiiTPL;
		}
		
		if Self::identify_scriptable(bin_data) {
			godot_print!("Identified Scriptable");
			return ObjectType::Scriptable;
		}
		
		if Self::identify_multi_scriptable(bin_data) {
			godot_print!("Identified MultiScriptable");
			return ObjectType::MultiScriptable;
		}
		
		// I don't like this... last statement should be returning Unsupported
		godot_print!("Could not identify");
		return ObjectType::Unsupported;
	}
	
	
	// =================================================================================
	// LOADING
	// =================================================================================
	
	
	fn load_resource_file(bin_data: Vec<u8>) -> Dictionary {
		let header_pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		let mut resource_dictionary: Dictionary = Dictionary::new();
		
		// For every sub object
		let mut object_number: usize = 0;
		for object in 0..header_pointers.len() {
			let object_bin_data: Vec<u8>;
			let dictionary: Dictionary;
			
			// If this is the last object
			if object == header_pointers.len() - 1 {
				object_bin_data = bin_data[header_pointers[object]..].to_vec();
			}
			
			else {
				object_bin_data = bin_data[header_pointers[object]..header_pointers[object + 1]].to_vec();
			}
			
			// Get and load per object type
			match Self::identify_object(&object_bin_data) {
				ObjectType::SpriteListSelect => {
					let pointers: Vec<usize> = Self::get_pointers(&object_bin_data, 0x00, false);
					let last_pointer = pointers[pointers.len() - 1];
					let sprites = Self::load_sprite_list(&object_bin_data);
					
					let bitmask = PackedByteArray::from(object_bin_data[last_pointer..].to_vec());
					
					let object_data: Gd<ObjectData> = Gd::from_init_fn(|base| {
						ObjectData {
							base: base,
							name: format!("Object #{}", object_number).into(),
							cells: array![],
							sprites: sprites,
							scripts: PackedByteArray::new(),
							palettes: Array::new(),
						}
					});
					
					dictionary = dict! {
						"type": "sprite_list_bitmask",
						"data": object_data,
						"bitmask": bitmask,
					};
				},
				
				ObjectType::SpriteList => {
					let sprites = Self::load_sprite_list(&object_bin_data);
					
					let object_data: Gd<ObjectData> = Gd::from_init_fn(|base| {
						ObjectData {
							base: base,
							name: format!("Object #{}", object_number).into(),
							cells: array![],
							sprites: sprites,
							scripts: PackedByteArray::new(),
							palettes: Array::new(),
						}
					});
					
					dictionary = dict! {
						"type": "sprite_list",
						"data": object_data,
					};
				},
				
				// Only used by archive_jpf.bin, for speed,
				// assume rather than try to ID each object
				ObjectType::MultiScriptable => {
					let scriptable_pointers: Vec<usize> = Self::get_pointers(
						&object_bin_data, 0x00, false);
					
					let mut scriptables: Dictionary = dict! {};
					
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
						
						scriptables.set(pointer as i64, Self::load_scriptable(scriptable_bin_data, pointer));
					}
				
					dictionary = dict! {
						"type": "multi_scriptable",
						"data": scriptables,
					};
				},
				
				ObjectType::Scriptable => {
					let object_data: Gd<ObjectData> = Self::load_scriptable(object_bin_data, object_number);
					
					if object_data.bind().name != "Player".into() {
						object_number += 1;
					}
				
					dictionary = dict! {
						"type": "scriptable",
						"data": object_data,
					};
				},
				
				_ => {
					godot_print!("\tPassing thru as Unsupported data");
					dictionary = dict! {
						"type": "unsupported",
						"data": PackedByteArray::from(object_bin_data),
					};
				},
			}
			
			resource_dictionary.set(object as u32, dictionary);
		}
		
		return dict! {
			"objects": resource_dictionary,
		};
	}
	
	
	// Single object
	fn load_scriptable(bin_data: Vec<u8>, number: usize) -> Gd<ObjectData> {
		let pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		
		let mut name = format!("Object #{}", number);
		let cells = Self::load_cells(&bin_data, &pointers);
		let sprites = Self::load_sprites(&bin_data, &pointers);
		let scripts = PackedByteArray::from(Self::load_scripts(&bin_data, &pointers));
		let palettes = Self::load_palettes(&bin_data, &pointers);
		
		if palettes.len() > 0 {
			name = "Player".into();
		}
		
		return Gd::from_init_fn(|base| {
			ObjectData {
				base: base,
				name: name.into(),
				cells: cells,
				sprites: sprites,
				scripts: scripts,
				palettes: palettes,
			}
		});
	}
	
	
	// =================================================================================
	// LOADING (GENERIC TO ALL OBJECTS)
	// =================================================================================
	
	
	fn load_cells(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Array<Gd<Cell>> {
		// Load cells
		let cell_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[0], false);
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
				_ => continue,
			}
		}
		
		return cells;
	}
	
	
	fn load_sprites(bin_data: &Vec<u8>, pointers: &Vec<usize>) -> Array<Gd<BinSprite>> {
		// Load sprites
		let sprite_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[1], false);
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
			
			match SpriteLoadSave::load_sprite_data(bin_data[start..end].to_vec()) {
				Some(sprite) => {
					sprites.push(&sprite);
				},
				
				None => {
					continue;
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
		
		let palette_pointers: Vec<usize> = Self::get_pointers(&bin_data, pointers[3], false);
		for palette in palette_pointers.iter() {
			let cursor: usize = pointers[3] + palette;
			let palette_data: Vec<u8> = bin_data[cursor..cursor + 0x410].to_vec();
			
			match BinPalette::from_bin_data(palette_data) {
				Some(palette) => palettes.push(&palette),
				None => continue,
			}
		}
		
		return palettes;
	}
	
	
	fn load_sprite_list(bin_data: &Vec<u8>) -> Array<Gd<BinSprite>> {
		let sprite_pointers: Vec<usize> = Self::get_pointers(&bin_data, 0x00, false);
		let mut sprites: Array<Gd<BinSprite>> = Array::new();
		
		for sprite in 0..sprite_pointers.len() {
			let sprite_data: Vec<u8>;
			let start: usize = sprite_pointers[sprite];
			
			if sprite < sprite_pointers.len() - 1 {
				let end = sprite_pointers[sprite + 1];
				sprite_data = bin_data[start..end].to_vec();
			}
			
			else {
				sprite_data = bin_data[start..].to_vec();
			}
			
			match SpriteLoadSave::load_sprite_data(sprite_data) {
				Some(sprite) => {
					sprites.push(&sprite);
				},
				
				None => {
					continue;
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