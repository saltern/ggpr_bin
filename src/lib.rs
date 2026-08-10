use godot::prelude::*;

// Gear Studio
pub mod bin_resource;
pub mod bin_identify;
pub mod bin_cell;
pub mod bin_sprite;
pub mod bin_script;
pub mod bin_palette;
pub mod bin_decrypt;
pub mod sprite_load_save;
pub mod sprite_import_export;

// Ghoul
pub mod sprite_get;
pub mod sprite_compress;
pub mod sprite_transform;

// Generic
pub mod sort;
pub mod psd;

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
	fn deserialize(bin_data: Vec<u8>) -> Gd<Self>;
}