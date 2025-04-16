use godot::prelude::*;

pub const ENCRYPTED_SIGNATURE: u32 = 0x41534743;
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


#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
pub enum ObjectType {
	Sprite,				// Good		-- Covered
	SpriteList,			// Good		-- Covered
	SpriteListSelect,	// Good		-- Covered
	JPFPlainText,		// Good		-- Covered, currently partially unsupported
	WiiTPL,				// Good		-- Covered, currently unsupported
	Scriptable,			// Good		-- Covered, currently partially unsupported
	MultiScriptable,	// Good		-- Covered
	Unsupported,
}
	
	
// =================================================================================
// UTILITY
// =================================================================================


pub fn get_pointers(bin_data: &Vec<u8>, mut cursor: usize, big_endian: bool) -> Vec<usize> {
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


pub fn identify_audio_wbnd(bin_data: &Vec<u8>) -> bool {
	let pointers: Vec<usize> = get_pointers(&bin_data, 0x00, true);
	return pointers[0] == WBND_SIGNATURE;
}


pub fn identify_audio_vagp(bin_data: &Vec<u8>) -> bool {
	let pointers: Vec<usize> = get_pointers(&bin_data, 0x00, true);
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


pub fn identify_sprite(bin_data: &Vec<u8>) -> bool {
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


pub fn identify_sprite_list_select(bin_data: &Vec<u8>) -> bool {
	// Checks first sprite only
	if !identify_sprite_list(bin_data) {
		return false;
	}
	
	let header_pointers: Vec<usize> = get_pointers(bin_data, 0x00, false);
	
	// Check bitmask
	let pointer_select: usize;
	let bitmask_end: usize;
	
	if header_pointers.len() > 179 {
		pointer_select = header_pointers[178];
		bitmask_end = header_pointers[179];
	} else {
		pointer_select = header_pointers[header_pointers.len() - 1];
		bitmask_end = bin_data.len();
	}
	
	let bitmask: Vec<u8> = bin_data[pointer_select..bitmask_end].to_vec();
	
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


pub fn identify_sprite_list(bin_data: &Vec<u8>) -> bool {
	let header_pointers: Vec<usize> = get_pointers(bin_data, 0x00, false);
	
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


pub fn identify_jpf_plain_text(bin_data: &Vec<u8>) -> bool {
	let header_pointers: Vec<usize> = get_pointers(&bin_data, 0x00, false);
	
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


pub fn identify_wii_tpl(bin_data: &Vec<u8>) -> bool {
	return u32::from_be_bytes([
		bin_data[0x00], bin_data[0x01], bin_data[0x02], bin_data[0x03],
	]) == WII_TPL_SIGNATURE;
}


pub fn identify_scriptable(bin_data: &Vec<u8>) -> bool {
	let header_pointers: Vec<usize> = get_pointers(bin_data, 0x00, false);
	
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
	if cell_pointers.len() < 2 {
		cell_pointers.push(header_pointers[1] - header_pointers[0]);
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


pub fn identify_multi_scriptable(bin_data: &Vec<u8>) -> bool {
	let header_pointers: Vec<usize> = get_pointers(bin_data, 0x00, false);
	
	for pointer in 0..header_pointers.len() {
		let pointers_this_object: Vec<usize> = get_pointers(
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


pub fn identify_object(bin_data: &Vec<u8>) -> ObjectType {
	if identify_audio_wbnd(bin_data) {
		return ObjectType::Unsupported;
	}
	
	if identify_audio_vagp(bin_data) {
		return ObjectType::Unsupported;
	}
	
	if identify_sprite(bin_data) {
		return ObjectType::Sprite;
	}
	
	if identify_sprite_list_select(bin_data) {
		return ObjectType::SpriteListSelect;
	}
	
	if identify_sprite_list(bin_data) {
		return ObjectType::SpriteList;
	}
	
	if identify_jpf_plain_text(bin_data) {
		return ObjectType::JPFPlainText;
	}
	
	if identify_wii_tpl(bin_data) {
		return ObjectType::WiiTPL;
	}
	
	if identify_scriptable(bin_data) {
		return ObjectType::Scriptable;
	}
	
	if identify_multi_scriptable(bin_data) {
		return ObjectType::MultiScriptable;
	}
	
	return ObjectType::Unsupported;
}