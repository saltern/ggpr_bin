use godot::prelude::*;

use crate::data_types::bin_sprite::BinSprite;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct SpriteSingle {
	base: Base<Resource>,
	sprite: Option<Gd<BinSprite>>,
}

#[godot_api]
impl IResource for SpriteSingle {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			sprite: None,
		}
	}
}