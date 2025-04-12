use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::path::PathBuf;
use std::ops::Deref;
use godot::classes::{Image, ImageTexture};
use serde::Serialize;
use serde::Deserialize;

use godot::prelude::*;
use godot::classes::image::Format;
use crate::bin_sprite::BinSprite;

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
			base,
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
	/// Array containing every box in this cell.
	#[export] pub boxes: Array<Gd<BoxInfo>>,
	/// The horizontal offset to display the sprite at.
	#[export] pub sprite_x_offset: i16,
	/// The vertical offset to display the sprite at.
	#[export] pub sprite_y_offset: i16,
	/// Unknown data. 4 bytes.
	#[export] pub unknown_1: u32,
	/// Sprite number to display.
	#[export] pub sprite_index: u16,
	/// Unknown data. 2 bytes.
	#[export] pub unknown_2: u16,
}


#[godot_api]
impl IResource for Cell {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
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
	
	
	/// Saves cell to a .json file.
	pub fn to_file(&self, path_buf: PathBuf) {
		let json_string = self.serialize();
		
		match fs::File::create(path_buf) {
			Ok(file) => {
				let ref mut buffer = BufWriter::new(file);
				let _ = buffer.write(json_string.as_bytes());
				let _ = buffer.flush();
			},
			
			_ => (),
		}
	}
	
	
	// Returns a binary representation of the cell.
	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();
		
		// Hitbox count
		bin_data.extend((self.boxes.len() as u32).to_le_bytes());
		
		// Hitbox data
		for hitbox in self.boxes.iter_shared() {
			let binding = hitbox.bind();
			bin_data.extend(binding.x_offset.to_le_bytes());
			bin_data.extend(binding.y_offset.to_le_bytes());
			bin_data.extend(binding.width.to_le_bytes());
			bin_data.extend(binding.height.to_le_bytes());
			bin_data.extend(binding.box_type.to_le_bytes());
			bin_data.push(binding.crop_x_offset);
			bin_data.push(binding.crop_y_offset);
		}
		
		// Rest of data
		bin_data.extend((self.sprite_x_offset).to_le_bytes());
		bin_data.extend((self.sprite_y_offset).to_le_bytes());
		bin_data.extend((self.unknown_1).to_le_bytes());
		bin_data.extend((self.sprite_index).to_le_bytes());
		bin_data.extend((self.unknown_2).to_le_bytes());
		
		// Pad and align
		while bin_data.len() % 0x10 != 0x00 {
			bin_data.push(0xFF);
		}
		
		return bin_data;
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
						base,
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
					base,
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


	// Cell constructor from raw binary data.
	pub fn from_binary_data(bin_data: &[u8]) -> Option<Gd<Self>> {
		if bin_data.len() < 0x04 {
			return None;
		}
	
		let hitbox_count: u32 = u32::from_le_bytes(
			[bin_data[0x00], bin_data[0x01], bin_data[0x02], bin_data[0x03]]
		);
		
		if bin_data.len() < (0x0C * hitbox_count as usize) + 0x10 {
			return None;
		}
		
		let mut hitbox_array: Array<Gd<BoxInfo>> = Array::new();
		for hitbox_number in 0..hitbox_count as usize {
			hitbox_array.push(&Gd::from_init_fn(|base| {
				BoxInfo {
					base: base,
					x_offset: i16::from_le_bytes([
						bin_data[0x0C * hitbox_number + 0x04],
						bin_data[0x0C * hitbox_number + 0x05]
					]),
					y_offset: i16::from_le_bytes([
						bin_data[0x0C * hitbox_number + 0x06],
						bin_data[0x0C * hitbox_number + 0x07]
					]),
					width: u16::from_le_bytes([
						bin_data[0x0C * hitbox_number + 0x08],
						bin_data[0x0C * hitbox_number + 0x09]
					]),
					height: u16::from_le_bytes([
						bin_data[0x0C * hitbox_number + 0x0A],
						bin_data[0x0C * hitbox_number + 0x0B]
					]),
					box_type: u16::from_le_bytes([
						bin_data[0x0C * hitbox_number + 0x0C],
						bin_data[0x0C * hitbox_number + 0x0D]
					]),
					crop_x_offset: bin_data[0x0C * hitbox_number + 0x0E],
					crop_y_offset: bin_data[0x0C * hitbox_number + 0x0F],
				}
			}));
		}
		
		let cursor: usize = (hitbox_count as usize) * 0x0C + 0x04;
		return Some(Gd::from_init_fn(|base| {
		Cell {
				base,
				boxes: hitbox_array,
				sprite_x_offset: i16::from_le_bytes([
					bin_data[cursor + 0x00], bin_data[cursor + 0x01]
				]),
				sprite_y_offset: i16::from_le_bytes([
					bin_data[cursor + 0x02], bin_data[cursor + 0x03]
				]),
				unknown_1: u32::from_le_bytes([
					bin_data[cursor + 0x04], bin_data[cursor + 0x05],
					bin_data[cursor + 0x06], bin_data[cursor + 0x07]
				]),
				sprite_index: u16::from_le_bytes([
					bin_data[cursor + 0x08], bin_data[cursor + 0x09]
				]),
				unknown_2: u16::from_le_bytes([
					bin_data[cursor + 0x0A], bin_data[cursor + 0x0B]
				]),
			}
		}));
	}


	/// Serializes the cell into a JSON string.
	pub fn serialize(&self) -> String {
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
			boxes,
			sprite_info,
		};
		
		return serde_json::to_string_pretty(&cell_json).unwrap();
	}
	
	
	/// Clamps the sprite index for this cell to the specified maximum.
	#[func] pub fn clamp_sprite_index(&mut self, sprite_max: u16) {
		self.sprite_index = self.sprite_index.clamp(0, sprite_max);
	}


	#[func] pub fn rebuild_sprite(&self, sprite: Gd<BinSprite>, visual_1: bool) -> Dictionary {
		let mut region_vec: Vec<Gd<BoxInfo>> = Vec::new();

		for box_type in [3u16, 6].iter() {
			for item in self.boxes.iter_shared() {
				let hitbox = item.bind();

				// Not a region box
				if &hitbox.box_type != box_type { continue; }

				region_vec.push(item.clone());
			}
		}

		if region_vec.is_empty() {
			return dict![];
		}

		// Results dictionary
		let mut dictionary: Dictionary = Dictionary::new();

		// Allocate pow2 texture like +R does...
		let sprite_ref = sprite.bind();
		let pow2_width = sprite_ref.width.next_power_of_two() as isize;
		let pow2_height = sprite_ref.height.next_power_of_two() as isize;

		let mut pow2_src_pixels: Vec<u8> = Vec::with_capacity((pow2_width * pow2_height) as usize);

		// Blit source sprite into pow2 texture
		for row in 0..sprite_ref.height as usize {
			for col in 0..sprite_ref.width as usize {
				pow2_src_pixels.push(sprite_ref.pixels[row * sprite_ref.width as usize + col]);
			}

			for _col in sprite_ref.width as isize..pow2_width {
				pow2_src_pixels.push(0x00);
			}
		}

		for _row in sprite_ref.height as isize..pow2_height {
			for _col in 0..pow2_width {
				pow2_src_pixels.push(0x00);
			}
		}

		for region_id in 0..region_vec.len() {
			let region = region_vec[region_id].bind();

			let region_x = region.x_offset as isize;
			let region_y = region.y_offset as isize;

			let sprite_width = sprite_ref.width as isize;
			let sprite_height = sprite_ref.height as isize;

			// Get sampling origin coords with weird +R mixed system
			let x_origin: isize;
			let y_origin: isize;

			if region.x_offset < 0 {
				x_origin = wrap(region_x, 0, sprite_width - 1);
			} else {
				x_origin = wrap(region_x, 0, pow2_width - 1);
			}

			if region.y_offset < 0 {
				y_origin = wrap(region_y, 0, sprite_height - 1);
			} else {
				y_origin = wrap(region_y, 0, pow2_height - 1);
			}

			// Allocate target pixel vector
			let mut pixel_vec: Vec<u8> = vec![0u8; region.width as usize * region.height as usize];

			for row in 0..region.height as isize {
				for col in 0..region.width as isize {
					let source_x = wrap(x_origin + col, 0, pow2_width - 1);
					let mut source_y = wrap(y_origin + row, 0, pow2_height - 1);

					if visual_1 && region.box_type == 6 {
						source_y += region.height as isize - row * 2;
					}

					let source = source_y * pow2_width + source_x;
					let target = row * region.width as isize + col;

					pixel_vec[target as usize] = pow2_src_pixels[source as usize];
				}
			}

			let image: Gd<Image> = Image::create_from_data(
				region.width as i32, region.height as i32, false, Format::L8, &PackedByteArray::from(pixel_vec)
			).unwrap();

			let texture: Gd<ImageTexture> = ImageTexture::create_from_image(&image).unwrap();

			let mut this_rect: Dictionary = Dictionary::new();
			this_rect.set("x_offset", region.x_offset);
			this_rect.set("y_offset", region.y_offset);
			this_rect.set("texture", texture);
			dictionary.set(region_id as u64, this_rect);
		}

		return dictionary;
	}
}


// Not my code
pub fn wrap(value: isize, min: isize, max: isize) -> isize {
	let (mut value, min, max) = (value as i128, min as i128, max as i128);
	let range_size = max - min + 1;

	if value < min {
		value += range_size * ((min - value) / range_size + 1);
	}

	return (min + (value - min) % range_size) as isize;
}