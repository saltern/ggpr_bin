use std::path::PathBuf;
use std::fs;
use std::fs::ReadDir;

use godot::prelude::*;


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Rust natord file name sorting access for Godot.
struct FileSort {
	base: Base<Resource>,
}


#[godot_api]
impl IResource for FileSort {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base: base,
		}
	}
}


#[godot_api]
impl FileSort {
	#[func]
	fn get_sorted_files(path: GString, extension: GString) -> Vec<GString> {
		let path_buf: PathBuf = PathBuf::from(path.to_string());
		
		if !path_buf.exists() {
			godot_print!("sort::FileSort::get_sorted_files() error: Could not find directory!");
			return Default::default();
		}
		
		let mut file_vector: Vec<PathBuf>;
		
		match fs::read_dir(path_buf) {
			Ok(value) => file_vector = get_files(value, &extension.to_string()),
			_ => return Default::default(),
		}
		
		file_vector.sort_by(|a, b| natord::compare(a.to_str().unwrap(), b.to_str().unwrap()));
		
		let mut return_vector: Vec<GString> = Vec::new();
		
		for item in file_vector {
			return_vector.push(GString::from(item.to_str().unwrap()));
		}
		
		return return_vector;
	}
}


fn get_files(read_dir: ReadDir, extension: &str) -> Vec<PathBuf> {
	let mut return_vector: Vec<PathBuf> = Vec::new();

	for entry in read_dir {
		let path = entry.unwrap().path();
		
		match path.extension() {
			Some(value) => {
				if value != extension {
					continue
				}
			},
			
			_ => continue,
		}
		return_vector.push(path);
	}
	return return_vector;
}