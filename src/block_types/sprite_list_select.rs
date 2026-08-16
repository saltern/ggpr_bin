use godot::prelude::*;

use crate::data_types::bin_sprite::BinSprite;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct CursorMask {
	base: Base<Resource>,
	width: u32,
	height: u32,
	pixels: Vec<u8>,
}

#[godot_api]
impl IResource for CursorMask {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			width: 0,
			height: 0,
			pixels: Vec::new(),
		}
	}
}

impl CursorMask {
	fn create(width: u32, height: u32, pixels: Vec<u8>) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				width,
				height,
				pixels,
			}
		});
	}
}

// -------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct SpriteListSelect {
	base: Base<Resource>,
	sprites: Array<Gd<BinSprite>>,
	cursor_mask: Gd<CursorMask>,
}

#[godot_api]
impl IResource for SpriteListSelect {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			sprites: Array::new(),
			cursor_mask: CursorMask::create(1, 1, vec![0u8]),
		}
	}
}