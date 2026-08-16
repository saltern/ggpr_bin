use crate::data_types::bin_sprite::BinSprite;
use crate::Serialization;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct GGDataFile {
	base: Base<Resource>,
	objects: Vec<Gd<Resource>>,
}

impl Serialization for Resource {
	fn serialize(&self) -> Vec<u8> {
		return Vec::new();
	}
}

#[godot_api]
impl IResource for GGDataFile {
	fn init(base: Base<Resource>) -> Self {
		let test = BinSprite::init_empty_palette();
		let mut vec: Vec::<Gd<Resource>> = Vec::new();
		vec.push(test as Resource);

		Self {
			base,
			objects: Vec::<Gd<Resource>>::new(),
		}
	}
}

impl Serialization for GGDataFile {
	fn serialize(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		for block in 0..self.objects.len() {
			let object = &self.objects[block];
			bin_data.extend(object.serialize());
		}

		return bin_data;
	}
}