use crate::{
	fs,
	PathBuf,
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
		
		let mut bin_data: Vec<u8>;
		let color_count: usize;
		
		match fs::read(path_buf) {
			Ok(data) => {
				if data.len() != 0x50 && data.len() != 0x410 {
					godot_print!("Invalid palette! {}", data.len());
					return Default::default();
				}
				
				color_count = 2usize.pow(data[0x04] as u32);
				bin_data = Vec::from(&data[0x10..]);
			},
			
			_ => {
				godot_print!("Could not load palette file!");
				return Default::default();
			},
		}
		
		let mut temp_pal: Vec<u8> = vec![0u8; bin_data.len()];
		
		// Reindex
		if color_count == 256 {
			for color in 0..color_count {
				let new_index: usize = transform_index(color);
				temp_pal[4 * color + 0] = bin_data[4 * new_index + 0];
				temp_pal[4 * color + 1] = bin_data[4 * new_index + 1];
				temp_pal[4 * color + 2] = bin_data[4 * new_index + 2];
				temp_pal[4 * color + 3] = bin_data[4 * new_index + 3];
			}
		}
		
		bin_data = temp_pal.clone();
		
		// Alpha processing
		for color in 0..color_count {
			let alpha: usize = 4 * color + 3;
			if bin_data[alpha] == 0x80 {
				bin_data[alpha] = 0xFF;
			}
			
			else {
				bin_data[alpha] *= 2;
			}
		}
		
		return Gd::from_init_fn(|base| {
			Self {
				base: base,
				palette: PackedByteArray::from(bin_data),
			}
		});
	}
}


fn transform_index(mut value: usize) -> usize {
	if ((value / 8) + 2) % 4 == 0 {
		value -= 8;
	}
	
	else if ((value / 8) + 3) % 4 == 0 {
		value += 8
	}
	
	return value;
}