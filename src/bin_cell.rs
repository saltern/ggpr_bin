use std::fs;
use std::path::PathBuf;
use std::ops::Deref;
use serde::Serialize;
use serde::Deserialize;

use godot::prelude::*;


#[derive(Serialize, Deserialize, Debug)]
struct CellJSONBox {
	x_offset: i16,
	y_offset: i16,
	width: u16,
	height: u16,
	box_type: u32,
}


#[derive(Serialize, Deserialize, Debug)]
struct CellJSONSpriteInfo {
	index: u16,
	unk: u32,
	x_offset: i16,
	y_offset: i16,
}


#[derive(Serialize, Deserialize, Debug)]
struct CellJSON {
	boxes: Vec<CellJSONBox>,
	sprite_info: CellJSONSpriteInfo,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Representation of a single hitbox. All values are in pixels.
pub struct BoxInfo {
	base: Base<Resource>,
	/// The box's X offset from the object's origin.
	#[export] pub x_offset: i16,
	/// The box's Y offset from the object's origin.
	#[export] pub y_offset: i16,
	/// The box's width.
	#[export] pub width: u16,
	/// The box's height.
	#[export] pub height: u16,
	/// The box's type.
	#[export] pub box_type: u16,
	/// The X offset to draw the cutout at for type 3/type 6 boxes.
	/// Multiplied by 8 in game.
	#[export] pub crop_x_offset: u8,
	/// The Y offset to draw the cutout at for type 3/type 6 boxes.
	/// Multiplied by 8 in game.
	#[export] pub crop_y_offset: u8,
}


#[godot_api]
impl IResource for BoxInfo {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
			x_offset: 0,
			y_offset: 0,
			width: 0,
			height: 0,
			box_type: 0,
			crop_x_offset: 0,
			crop_y_offset: 0,
		}
	}
}


#[derive(GodotClass, Debug)]
#[class(tool, base=Resource)]
/// Representation of a Guilty Gear cell (a combination of a sprite and hitboxes).
pub struct Cell {
	base: Base<Resource>,
	#[export] pub boxes: Array<Gd<BoxInfo>>,
	#[export] pub sprite_x_offset: i16,
	#[export] pub sprite_y_offset: i16,
	#[export] pub unknown_1: u32,
	#[export] pub sprite_index: u16,
	#[export] pub unknown_2: u16,
}


#[godot_api]
impl IResource for Cell {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
			boxes: Array::new(),
			sprite_x_offset: 0,
			sprite_y_offset: 0,
			unknown_1: 0,
			sprite_index: 0,
			unknown_2: 0,
		}
	}
}


#[godot_api]
impl Cell {
	/// Cell constructor from .json files.
	pub fn from_file(path_buf: PathBuf) -> Option<Gd<Self>> {
		match fs::read_to_string(&path_buf) {
			Ok(string) => return Self::from_json_string(string),
			_ => return None,
		}
	}
	
	
	/// Cell constructor from .json strings.
	pub fn from_json_string(string: String) -> Option<Gd<Self>> {
		let cell_json: CellJSON;

		match serde_json::from_str(&string) {
			Ok(cell) => cell_json = cell,
			_ => return None,
		}
		
		// Get hitboxes
		let mut hitbox_array: Array<Gd<BoxInfo>> = Array::new();
		for hitbox in cell_json.boxes.iter() {
			let new_hitbox: Gd<BoxInfo> = Gd::from_init_fn(
				|base| {
					BoxInfo {
						base: base,
						x_offset: hitbox.x_offset,
						y_offset: hitbox.y_offset,
						width: hitbox.width,
						height: hitbox.height,
						box_type: (hitbox.box_type & 0xFFFF) as u16,
						crop_x_offset: ((hitbox.box_type >> 16) & 0xFF) as u8,
						crop_y_offset: ((hitbox.box_type >> 24) & 0xFF) as u8,
					}
				}
			);
			hitbox_array.push(&new_hitbox);
		}
		
		let cell: Gd<Self> = Gd::from_init_fn(
			|base| {
				Self {
					base: base,
					boxes: hitbox_array,
					sprite_x_offset: cell_json.sprite_info.x_offset,
					sprite_y_offset: cell_json.sprite_info.y_offset,
					unknown_1: cell_json.sprite_info.unk,
					sprite_index: cell_json.sprite_info.index,
					unknown_2: 0,
				}
			}
		);
		
		return Some(cell);
	}


	/// Serializes the cell into a JSON string.
	pub fn serialize(&mut self) -> String {
		let mut boxes: Vec<CellJSONBox> = Vec::new();
		
		for mut hitbox in self.boxes.iter_shared() {
			let box_mut = hitbox.bind_mut();
			let this_box: &BoxInfo = box_mut.deref();
			
			let json_box: CellJSONBox = CellJSONBox {
				x_offset: this_box.x_offset,
				y_offset: this_box.y_offset,
				width: this_box.width,
				height: this_box.height,
				box_type: (this_box.box_type as u32) | (this_box.crop_x_offset as u32) << 16 | (this_box.crop_y_offset as u32) << 24,
			};
			
			boxes.push(json_box);
		}
		
		let sprite_info = CellJSONSpriteInfo {
			index: self.sprite_index,
			unk: self.unknown_1,
			x_offset: self.sprite_x_offset,
			y_offset: self.sprite_y_offset,
		};
		
		let cell_json = CellJSON {
			boxes: boxes,
			sprite_info: sprite_info,
		};
		
		return serde_json::to_string(&cell_json).unwrap();
	}
}