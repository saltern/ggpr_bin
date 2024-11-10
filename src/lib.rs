use godot::prelude::*;

pub mod bin_resource;
pub mod bin_cell;
pub mod bin_sprite;
pub mod bin_palette;
pub mod sprite_get;
pub mod sprite_compress;
pub mod sprite_transform;
pub mod sprite_load_save;
pub mod sprite_import_export;
pub mod sort;

struct GodotGhoul;

#[gdextension]
unsafe impl ExtensionLibrary for GodotGhoul {}