use godot::prelude::*;

use crate::data_types::bin_sprite::BinSprite;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct JPFPlainText {
	base: Base<Resource>,
	char_index: Vec<u8>,
	sprites: Array<Gd<BinSprite>>,
}

#[godot_api]
impl IResource for JPFPlainText {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			char_index: Vec::new(),
			sprites: Array::new(),
		}
	}
}