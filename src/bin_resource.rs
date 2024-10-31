use std::fs;
use std::fs::File;
use std::path::PathBuf;

use godot::prelude::*;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust GGXXAC+R binary resource file loader.
struct ResourceLoadSave {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for ResourceLoadSave {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl ResourceLoadSave {
	/// Loads a BIN resource file, returning the objects contained within.
}