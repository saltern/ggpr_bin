use godot::prelude::*;

#[derive(GodotClass)]
#[class(tool, base=Resource)]
pub struct Unsupported {
	base: Base<Resource>,
	data: Vec<u8>,
}

#[godot_api]
impl IResource for Unsupported {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			data: Vec::new(),
		}
	}
}