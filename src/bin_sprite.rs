use godot::prelude::*;
use godot::classes::ImageTexture;
use godot::classes::Image;
use godot::classes::image::Format;

use crate::sprite_transform;
use crate::sprite_compress;
use crate::sprite_compress::CompressedData;
use crate::sprite_compress::SpriteData;

pub const HEADER_SIZE: usize = 16;

pub struct BinHeader {
	pub compressed: bool,
	pub clut: u16,
	pub bit_depth: u16,
	pub width: u16,
	pub height: u16,
	pub tw: u16,
	pub th: u16,
	pub hash: u16,
}


pub fn get_header(data: Vec<u8>) -> BinHeader {	
	return BinHeader {
		compressed: data[0] == 1,
		
		clut: u16::from_le_bytes([
			data[0x02], data[0x03]
		]),
			
		bit_depth: u16::from_le_bytes([
			data[0x04], data[0x05]
		]),
		
		width: u16::from_le_bytes([
			data[0x06], data[0x07]
		]),
		
		height: u16::from_le_bytes([
			data[0x08], data[0x09]
		]),
		
		tw: u16::from_le_bytes([
			data[0x0A], data[0x0B]
		]),
		
		th: u16::from_le_bytes([
			data[0x0C], data[0x0D]
		]),
		
		hash: u16::from_le_bytes([
			data[0x0E], data[0x0F]
		]),
	}
}


pub fn make_header(compressed: bool, clut: u16, bit_depth: u16, width: u16, height: u16, tw: u16, th: u16, hash: u16) -> Vec<u8> {
	let mut return_vector: Vec<u8> = Vec::with_capacity(0x10);
	
	// mode (compressed/uncompressed)
	return_vector.push(compressed as u8);
	return_vector.push(0x00);
	
	// clut (embedded palette)
	return_vector.extend_from_slice(&clut.to_le_bytes());
	
	// pix (bit depth)
	return_vector.extend_from_slice(&bit_depth.to_le_bytes());
	
	// width
	return_vector.extend_from_slice(&width.to_le_bytes());
	
	// height
	return_vector.extend_from_slice(&height.to_le_bytes());
	
	// tw (unknown)
	return_vector.extend_from_slice(&tw.to_le_bytes());
	
	// th (unknown)
	return_vector.extend_from_slice(&th.to_le_bytes());
	
	// hash (generation method unknown, doesn't affect result)
	return_vector.extend_from_slice(&hash.to_le_bytes());
	
	return return_vector;
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Data resulting from loading a sprite_#.bin file.
pub struct BinSprite {
	base: Base<Resource>,
	/// A [PackedByteArray] representing the raw pixel vector.
	#[export]
	pub pixels: PackedByteArray,
	/// The sprite's width.
	#[export]
	pub width: u16,
	/// The sprite's height.
	#[export]
	pub height: u16,
	/// The base image loaded from a sprite.
	#[export]
	pub image: Option<Gd<Image>>,
	/// The grayscale texture loaded from a sprite.
	#[export]
	pub texture: Option<Gd<ImageTexture>>,
	/// The sprite's color depth.
	#[export]
	pub bit_depth: u16,
	/// A [PackedByteArray] representing a list of RGBA colors.
	#[export]
	pub palette: PackedByteArray,
}


#[godot_api]
impl IResource for BinSprite {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			pixels: PackedByteArray::from(vec![]),
			width: 0,
			height: 0,
			image: None,
			texture: None,
			bit_depth: 8,
			palette: PackedByteArray::from(vec![]),
		}
	}
}


#[godot_api]
impl BinSprite {
	/// Static constructor for BinSprites.
	#[func]
	pub fn new_from_data(
		pixels: PackedByteArray, width: u16, height: u16, image: Gd<Image>, bit_depth: u16,
		palette: PackedByteArray
	) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				pixels,
				width,
				height,
				texture: ImageTexture::create_from_image(&image),
				image: Some(image),
				bit_depth,
				palette,
			}
		});
	}
	
	
	pub fn to_bin(&self) -> Vec<u8> {
		let image = self.image.as_ref().unwrap();
		let width: u16 = image.get_width() as u16;
		let height: u16 = image.get_height() as u16;
	
		let sprite_data: SpriteData = SpriteData {
			width,
			height,
			bit_depth: self.bit_depth,
			pixels: self.pixels.to_vec(),
			palette: self.palette.to_vec(),
		};
		
		let compressed_data: CompressedData = sprite_compress::compress(sprite_data);
		
		// Generate hash
		let mut hash: u16 = 0;
		
		for byte in 0..compressed_data.stream.len() / 2 {
			hash = hash ^ (
				(compressed_data.stream[byte + 0] as u16) |
				(compressed_data.stream[byte + 1] as u16) << 8
			);
		}
		
		// Construct header
		let header: Vec<u8> = make_header(
			true,
			0x20 * (self.palette.len() != 0) as u16,
			self.bit_depth,
			width,
			height,
			0x0000,
			0x0000,
			hash,
		);
		
		let mut bin_data: Vec<u8> = Vec::new();
		let iterations_u32: u32 = compressed_data.iterations as u32;
		
		bin_data.extend(header);
		bin_data.extend(self.palette.to_vec());
		bin_data.extend_from_slice(&[
			(iterations_u32 >> 16) as u8,	// BB
			(iterations_u32 >> 24) as u8,	// AA
			(iterations_u32 >> 00) as u8,	// DD
			(iterations_u32 >> 08) as u8,	// CC
		]);
		
		for byte in 0..compressed_data.stream.len() / 2 {
			bin_data.extend([
				compressed_data.stream[2 * byte + 1],
				compressed_data.stream[2 * byte + 0],
			]);
		}
		
		return bin_data;
	}
	
	
	/// Reindexing function. Reorders colors from 1-2-3-4 to 1-3-2-4 and vice-versa.
	#[func]
	pub fn reindex(&mut self) {
		let new_pixels: Vec<u8> = sprite_transform::reindex_vector(self.pixels.to_vec());
		self.pixels = new_pixels.into();
		
		// Reconstruct image for preview in Godot
		let old_image: &Image = self.image.as_ref().unwrap();
		let tex_width: i32 = old_image.get_width() as i32;
		let tex_height: i32 = old_image.get_height() as i32;
		
		let new_image: Gd<Image>;
		
		match Image::create_from_data(
			tex_width,
			tex_height,
			// Mipmapping
			false,
			// Grayscale format
			Format::L8,
			// Pixel array
			&PackedByteArray::from(self.pixels.clone())
		) {
			Some(gd_image) => new_image = gd_image,
			_ => return,
		}
		
		self.image = Some(new_image.clone());
		self.texture = Some(ImageTexture::create_from_image(&new_image).unwrap());
	}
}