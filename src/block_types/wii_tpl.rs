use godot::prelude::*;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct WiiTPL {
	base: Base<Resource>,
	data: Vec<u8>,
}

#[godot_api]
impl IResource for WiiTPL {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			data: Vec::new(),
		}
	}
}