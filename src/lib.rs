use godot::prelude::*;

// Gear Studio
pub mod bin_decrypt;
pub mod sprite_importer;
pub mod sprite_exporter;

// Ghoul
pub mod sprite_transform;

// Generic
pub mod sort;
pub mod psd;
pub mod sprite_compress;

struct GGPRBin;

#[gdextension]
unsafe impl ExtensionLibrary for GGPRBin {}