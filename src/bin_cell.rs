use std::io::Write;
use std::io::BufWriter;
use std::fs;
use std::path::PathBuf;
use std::ops::Deref;
use std::cmp::{min, max};
use godot::classes::{Image, ImageTexture};
use serde::Serialize;
use serde::Deserialize;

use godot::prelude::*;
use godot::classes::image::Format;
use godot::meta::AsArg;
use crate::bin_sprite::BinSprite;
use crate::sprite_transform;
use crate::psd;

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
	#[export] pub crop_x_offset: i8,
	/// The Y offset to draw the cutout at for type 3/type 6 boxes.
	/// Multiplied by 8 in game.
	#[export] pub crop_y_offset: i8,
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
			bin_data.extend(binding.crop_x_offset.to_le_bytes());
			bin_data.extend(binding.crop_y_offset.to_le_bytes());
		}
		
		// Rest of data
		bin_data.extend(self.sprite_x_offset.to_le_bytes());
		bin_data.extend(self.sprite_y_offset.to_le_bytes());
		bin_data.extend(self.unknown_1.to_le_bytes());
		bin_data.extend(self.sprite_index.to_le_bytes());
		bin_data.extend(self.unknown_2.to_le_bytes());
		
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
						crop_x_offset: ((hitbox.box_type >> 16) & 0xFF) as i8,
						crop_y_offset: ((hitbox.box_type >> 24) & 0xFF) as i8,
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
					base,
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
					crop_x_offset: bin_data[0x0C * hitbox_number + 0x0E] as i8,
					crop_y_offset: bin_data[0x0C * hitbox_number + 0x0F] as i8,
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


	/// Rebuilds the sprite from type 3/6 boxes if present. Otherwise, returns the texture as-is.
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

		let sprite_ref = sprite.bind();

		// No regions. Return whole unmodified texture.
		// Is cloning bad? Doesn't seem slow.
		if region_vec.is_empty() {
			let mut return_dict = Dictionary::new();
			let mut single_dict = Dictionary::new();

			single_dict.set("x_offset", 0);
			single_dict.set("y_offset", 0);
			single_dict.set("texture", sprite_ref.texture.clone());
			return_dict.set(0, single_dict);

			return return_dict;
		}

		// Results dictionary
		let mut dictionary: Dictionary = Dictionary::new();

		// Allocate pow2 texture like +R does...
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

		// Loop through region boxes
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

			// Copy
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
				region.width as i32, region.height as i32, false, Format::L8,
				&PackedByteArray::from(pixel_vec)
			).unwrap();

			let texture: Gd<ImageTexture> = ImageTexture::create_from_image(&image).unwrap();

			let mut this_rect: Dictionary = Dictionary::new();
			this_rect.set("x_offset", region.x_offset + region.crop_x_offset as i16 * 8);
			this_rect.set("y_offset", region.y_offset + region.crop_y_offset as i16 * 8);
			this_rect.set("texture", texture);
			dictionary.set(region_id as u64, this_rect);
		}

		return dictionary;
	}


	fn get_extents(
		&self, sprite: &Gd<BinSprite>, box_types: &Array<bool>, origin: &PackedByteArray,
	) -> (isize, isize, isize, isize) {
		let mut u: isize = isize::MAX;
		let mut d: isize = isize::MIN;
		let mut l: isize = isize::MAX;
		let mut r: isize = isize::MIN;

		let sprite_ref = sprite.bind();

		let mut have_type_3: bool = false;

		// Sprites are always offset (-128,-128)
		let sprite_x = -128 + self.sprite_x_offset as isize;
		let sprite_y = -128 + self.sprite_y_offset as isize;

		// Hitboxes
		for hitbox in self.boxes.iter_shared() {
			let this_box = hitbox.bind();

			let box_x = this_box.x_offset as isize;
			let box_y = this_box.y_offset as isize;
			let box_w = this_box.width as isize;
			let box_h = this_box.height as isize;

			if this_box.box_type == 3 || this_box.box_type == 6 {
				have_type_3 = true;

				let crop_x_offset = this_box.crop_x_offset as isize * 8;
				let crop_y_offset = this_box.crop_y_offset as isize * 8;

				let sprite_piece_x = box_x + crop_x_offset + sprite_x;
				let sprite_piece_y = box_y + crop_y_offset + sprite_y;

				// Factor in type 3/6 sprite pieces
				u = min(u, sprite_piece_y);
				d = max(d, sprite_piece_y + box_h);
				l = min(l, sprite_piece_x);
				r = max(r, sprite_piece_x + box_w)
			}

			// Skip disabled box types in extents calculation
			let display_index = min(this_box.box_type as usize, box_types.len() - 1);
			if !box_types.at(display_index) {
				continue;
			}

			u = min(u, box_y);
			d = max(d, box_y + box_h);
			l = min(l, box_x);
			r = max(r, box_x + box_w);
		}

		// Sprite
		if !have_type_3 {
			u = min(u, sprite_y);
			d = max(d, sprite_y + sprite_ref.height as isize);
			l = min(l, sprite_x);
			r = max(r, sprite_x + sprite_ref.width as isize);
		}

		// Origin
		if !origin.is_empty() {
			let origin_w = origin[0] as isize;
			let origin_offset = origin_w / 2;
			d = max(d, origin_offset + 1);
			r = max(r, origin_offset + 1);
		}

		return(u, d, l, r);
	}


	fn layer_write_sprite(
		mut input_layer: Vec<u8>, min_x: isize, min_y: isize, image_w: isize,
		sprite_pieces: Dictionary, sprite_x: i16, sprite_y: i16, bit_depth: u16,
		palette: PackedByteArray
	) -> Vec<u8> {
		for key in sprite_pieces.keys_array().iter_shared() {
			let piece: Dictionary = sprite_pieces.at(key).to();
			// "x_offset", "y_offset", "texture"

			let mut at_x: i64 = piece.at("x_offset").to();
			let mut at_y: i64 = piece.at("y_offset").to();
			at_x += sprite_x as i64 - 128 - min_x as i64;
			at_y += sprite_y as i64 - 128 - min_y as i64;

			let piece_texture: Gd<ImageTexture> = piece.at("texture").to();
			let piece_image: Gd<Image> = piece_texture.get_image().unwrap();
			let piece_pixels: Vec<u8> = piece_image.get_data().to_vec();

			let piece_w = piece_texture.get_width() as isize;
			let piece_h = piece_texture.get_height() as isize;

			for row in 0..piece_h {
				for col in 0..piece_w {
					let index = row * piece_w + col;
					let mut pal_index: usize = piece_pixels[index as usize] as usize;

					if bit_depth == 8 {
						pal_index = sprite_transform::transform_index(pal_index as u8) as usize;
					}

					pal_index *= 4;

					let r: u8 = palette[pal_index + 0];
					let g: u8 = palette[pal_index + 1];
					let b: u8 = palette[pal_index + 2];
					let mut a: u8 = palette[pal_index + 3];

					if a < 128 {
						a *= 2;
					} else {
						a = 255;
					}

					let at = 4 * (
						(at_y as isize + row) * image_w + at_x as isize + col
					) as usize;

					input_layer[at + 0] = r;
					input_layer[at + 1] = g;
					input_layer[at + 2] = b;
					input_layer[at + 3] = a;
				}
			}
		}

		return input_layer;
	}


	fn layer_write_boxes(
		mut input_layer: Vec<u8>, min_x: isize, min_y: isize, image_w: isize,
		boxes: &Array<Gd<BoxInfo>>, box_display_types: Array<bool>,
		box_colors: &Array<Color>, box_thickness: i64
	) -> (Vec<u8>, usize) {
		let mut boxes_written: usize = 0;

		for hitbox in boxes.iter_shared() {
			let hitbox_ref = hitbox.bind();

			let display_index = min(hitbox_ref.box_type as usize, box_display_types.len() - 1);
			if !box_display_types.at(display_index) {
				continue;
			}

			boxes_written += 1;

			let this_color: Color = box_colors.at(hitbox_ref.box_type as usize);

			let box_x = hitbox_ref.x_offset as isize - min_x;
			let box_y = hitbox_ref.y_offset as isize - min_y;
			let box_w = hitbox_ref.width as isize;
			let box_h = hitbox_ref.height as isize;

			for t in 0..box_thickness as isize {
				// Top, bottom
				for x in box_x..box_x + box_w {
					let at_u = 4 * ((box_y + t) * image_w + x) as usize;
					let at_d = 4 * ((box_y + box_h - 1 - t) * image_w + x) as usize;

					input_layer[at_u + 0] = this_color.r8();
					input_layer[at_u + 1] = this_color.g8();
					input_layer[at_u + 2] = this_color.b8();
					input_layer[at_u + 3] = this_color.a8();

					input_layer[at_d + 0] = this_color.r8();
					input_layer[at_d + 1] = this_color.g8();
					input_layer[at_d + 2] = this_color.b8();
					input_layer[at_d + 3] = this_color.a8();
				}

				// Left, right
				for y in box_y..box_y + box_h {
					let mod_y = y * image_w;
					let at_l = 4 * (mod_y + box_x + t) as usize;
					let at_r = 4 * (mod_y + box_x + box_w - 1 - t) as usize;

					input_layer[at_l + 0] = this_color.r8();
					input_layer[at_l + 1] = this_color.g8();
					input_layer[at_l + 2] = this_color.b8();
					input_layer[at_l + 3] = this_color.a8();

					input_layer[at_r + 0] = this_color.r8();
					input_layer[at_r + 1] = this_color.g8();
					input_layer[at_r + 2] = this_color.b8();
					input_layer[at_r + 3] = this_color.a8();
				}
			}
		}

		return (input_layer, boxes_written);
	}


	fn layer_write_origin(
		mut input_layer: Vec<u8>, min_x: isize, min_y: isize, image_w: isize,
		origin: PackedByteArray
	) -> Vec<u8> {
		let origin_w = origin[0] as isize;
		let origin_pixels: Vec<u8> = origin.to_vec();
		let origin_offset: isize = (origin_w - 1) / 2;

		let at_x = -min_x - origin_offset;
		let at_y = -min_y - origin_offset;

		for row in 0..origin_w {
			for col in 0..origin_w {
				let from = 1 + 4 * (row * origin_w + col) as usize;

				// Skip transparent
				if origin_pixels[from + 3] == 0 {
					continue;
				}

				let at = 4 * (((at_y + row) * image_w) + at_x + col) as usize;

				input_layer[at + 0] = origin_pixels[from + 0];
				input_layer[at + 1] = origin_pixels[from + 1];
				input_layer[at + 2] = origin_pixels[from + 2];
				input_layer[at + 3] = origin_pixels[from + 3];
			}
		}

		return input_layer;
	}
	

	/// Saves the cell as a PNG snapshot.
	#[func] pub fn save_snapshot_png(
		// Sprite params
		&self, sprite: Gd<BinSprite>, palette: PackedByteArray, visual_1: bool,
		// Box params
		box_display_types: Array<bool>, box_colors: Array<Color>, box_thickness: i64,
		// Origin point params
		origin: PackedByteArray,
		// Save path
		path: GString
	) {
		let (min_y, max_y, min_x, max_x) = self.get_extents(&sprite, &box_display_types, &origin);
		let sprite_clone = sprite.clone();
		let sprite_ref = sprite_clone.bind();
		let sprite_pieces: Dictionary = self.rebuild_sprite(sprite, visual_1);

		let image_w = max_x - min_x;
		let image_h = max_y - min_y;

		// RGBA; 4 channels
		let mut pixel_vector: Vec<u8> = vec![0u8; (4 * image_w * image_h) as usize];

		// Write sprite
		pixel_vector = Self::layer_write_sprite(
			pixel_vector, min_x, min_y, image_w,
			sprite_pieces, self.sprite_x_offset, self.sprite_y_offset, sprite_ref.bit_depth,
			palette
		);

		// Write boxes
		pixel_vector = Self::layer_write_boxes(
			pixel_vector, min_x, min_y, image_w,
			&self.boxes, box_display_types, &box_colors, box_thickness
		).0;

		// Write origin cross
		if !origin.is_empty() {
			pixel_vector = Self::layer_write_origin(pixel_vector, min_x, min_y, image_w, origin);
		}

		let result = Image::create_from_data(
			image_w as i32, image_h as i32, false, Format::RGBA8,
			&PackedByteArray::from(pixel_vector)
		).unwrap();

		result.save_png(path.into_arg());
	}


	/// Saves the cell as a PSD snapshot.
	#[func] pub fn save_snapshot_psd(
		// Sprite params
		&self, sprite: Gd<BinSprite>, palette: PackedByteArray, visual_1: bool,
		// Box params
		box_display_types: Array<bool>, box_colors: Array<Color>, box_thickness: i64,
		// Origin point params
		origin: PackedByteArray,
		// Save path
		path: String
	) {
		let (min_y, max_y, min_x, max_x) = self.get_extents(&sprite, &box_display_types, &origin);
		let sprite_clone = sprite.clone();
		let sprite_ref = sprite_clone.bind();
		let sprite_pieces = self.rebuild_sprite(sprite, visual_1);

		let image_w = max_x - min_x;
		let image_h = max_y - min_y;

		// Layers
		let mut layer_vector: Vec<(&str, Vec<u8>)> = Vec::new();
		let layer_size = (4 * image_w * image_h) as usize;

		// Sprite
		let mut sprite_layer: Vec<u8> = vec![0u8; layer_size];
		sprite_layer = Self::layer_write_sprite(
			sprite_layer, min_x, min_y, image_w,
			sprite_pieces, self.sprite_x_offset, self.sprite_y_offset, sprite_ref.bit_depth,
			palette
		);

		layer_vector.push(("Sprite", sprite_layer));

		// Boxes, one layer per visible type
		for box_type_id in 0..box_display_types.len() {
			// Skip disabled types
			if !box_display_types.at(box_type_id) {
				continue;
			}

			let mut current_display_array: Array<bool> = array![
				false, false, false, false, false, false, false, false
			];

			current_display_array.set(box_type_id, true);
			let this_layer: Vec<u8> = vec![0u8; layer_size];
			let layer_name: &str;

			match box_type_id {
				0 | 1 => layer_name = "Hitboxes",
				2 => layer_name = "Hurtboxes",
				3 => layer_name = "Sprite regions (back)",
				4 => layer_name = "Collision extension",
				5 => layer_name = "Spawn points",
				6 => layer_name = "Sprite regions (front)",
				_ => layer_name = "Unknown type boxes"
			}

			let (this_layer, box_count) = Self::layer_write_boxes(
				this_layer, min_x, min_y, image_w,
				&self.boxes, current_display_array, &box_colors, box_thickness
			);

			if box_count == 0 {
				continue;
			}

			layer_vector.push((layer_name, this_layer));
		}

		// Origin point
		if !origin.is_empty() {
			let mut origin_layer: Vec<u8> = vec![0u8; layer_size];
			origin_layer = Self::layer_write_origin(origin_layer, min_x, min_y, image_w, origin);
			layer_vector.push(("Origin point", origin_layer));
		}

		// Have all layers
		psd::make_psd(image_w, image_h, layer_vector, path.to_string());
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