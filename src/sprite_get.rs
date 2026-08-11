use std::fs;
use std::fs::File;
use std::path::PathBuf;

use bmp_rust::bmp::{BMP, BITMAPFILEHEADER, DIBHEADER};

use godot::prelude::*;

use crate::bin_sprite;
//use crate::sprite_compress;
use crate::sprite_transform;

// use bin_sprite::BinHeader;
use bin_sprite::BinSprite;
//use sprite_compress::SpriteData;