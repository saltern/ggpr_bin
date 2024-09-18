use crate::{
	fs,
	PathBuf,
	sprite_transform,
	godot_api,
	godot_print,
	Gd,
	PackedByteArray,
	GString,
	GodotClass,
	Base,
	Resource,
	IResource,
};


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Color palette obtained from loading a palette_#.bin file.
pub struct BinPalette {
	base: Base<Resource>,
	/// The color palette loaded from the file.
	#[export]
	palette: PackedByteArray,
}


#[godot_api]
impl IResource for BinPalette {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
			palette: PackedByteArray::from(vec![]),
		}
	}
}


#[godot_api]
impl BinPalette {
	/// Static constructor for BinPalettes from .bin files.
	#[func]
	pub fn from_file(path: GString) -> Gd<Self> {
		let path_str: String = String::from(path);
		let path_buf: PathBuf = PathBuf::from(path_str);
		
		if !path_buf.exists() {
			godot_print!("Could not find palette file!");
			return Default::default();
		}
		
		let bin_data: Vec<u8>;
		
		match fs::read(path_buf) {
			Ok(data) => {
				if data.len() != 0x50 && data.len() != 0x410 {
					godot_print!("Invalid palette! {}", data.len());
					return Default::default();
				}
				
				bin_data = Vec::from(&data[0x10..]);
			},
			
			_ => {
				godot_print!("Could not load palette file!");
				return Default::default();
			},
		}
		
		return Gd::from_init_fn(|base| {
			Self {
				base: base,
				palette: PackedByteArray::from(bin_data),
			}
		});
	}
	
	/// Reindexing function. Reorders colors from 1-2-3-4 to 1-3-2-4 and vice-versa.
	#[func]
	pub fn reindex(&mut self) {		
		let mut temp_pal: Vec<u8> = vec![0u8; self.palette.len()];
		
		let color_count: usize = self.palette.len() / 4;
		
		for color in 0..color_count {
			let new_index: usize = sprite_transform::transform_index(color as u8) as usize;
			temp_pal[4 * color + 0] = self.palette[4 * new_index + 0];
			temp_pal[4 * color + 1] = self.palette[4 * new_index + 1];
			temp_pal[4 * color + 2] = self.palette[4 * new_index + 2];
			temp_pal[4 * color + 3] = self.palette[4 * new_index + 3];
		}
		
		self.palette = PackedByteArray::from(temp_pal);
	}
	
	/// Alpha halving function. Halves all alpha values except for 0xFF, which is set to 0x80.
	#[func]
	pub fn alpha_halve(&mut self) {
		self.palette = sprite_transform::alpha_halve(self.palette.to_vec()).into();
	}
	
	/// Alpha doubling function. Doubles all alpha values except for 0x80, which is set to 0xFF.
	#[func]
	pub fn alpha_double(&mut self) {
		self.palette = sprite_transform::alpha_double(self.palette.to_vec()).into();
	}
}