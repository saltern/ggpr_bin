use godot::prelude::*;

use crate::data_types::bin_script::BinScript;
use crate::data_types::bin_cell::Cell;
use crate::data_types::bin_sprite::BinSprite;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct Scriptable {
	base: Base<Resource>,
	cells: Array<Gd<Cell>>,
	sprites: Array<Gd<BinSprite>>,
	script: Option<Gd<BinScript>>,
	palettes: Array<Gd<BinSprite>>,
}

#[godot_api]
impl IResource for Scriptable {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			cells: Array::new(),
			sprites: Array::new(),
			script: None,
			palettes: Array::new(),
		}
	}
}

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct MultiScriptable {
	base: Base<Resource>,
	scriptables: Array<Gd<Scriptable>>,
}

#[godot_api]
impl IResource for MultiScriptable {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			scriptables: Array::new(),
		}
	}
}