use godot::prelude::*;

use crate::data_types::bin_sprite::BinSprite;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct SpriteList {
	base: Base<Resource>,
	sprites: Array<Gd<BinSprite>>,
}

#[godot_api]
impl IResource for SpriteList {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			sprites: Array::new(),
		}
	}
}