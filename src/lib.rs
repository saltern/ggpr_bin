use godot::prelude::*;

// Gear Studio
//pub mod bin_resource;
//pub mod bin_identify;
pub mod bin_decrypt;
pub mod sprite_import;
//pub mod sprite_load_save;
//pub mod sprite_import_export;

// Files
//pub mod gg_data_file;
//pub mod block_types;

// Data
//pub mod data_types;

// Ghoul
pub mod sprite_transform;

// Generic
pub mod sort;
pub mod psd;
pub mod sprite_compress;

struct GGPRBin;

#[gdextension]
unsafe impl ExtensionLibrary for GGPRBin {}


pub trait Identification {
	fn identify(bin_data: &Vec<u8>) -> bool;
}


pub trait Serialization {
	fn serialize(&self) -> Vec<u8>;
}


pub trait Deserialization {
	fn deserialize(bin_data: &Vec<u8>) -> Option<Gd<Self>> where Self: GodotClass;
}