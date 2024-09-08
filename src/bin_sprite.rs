use crate::{
	godot_api,
	Gd,
	PackedByteArray,
	GodotClass,
	Base,
	Resource,
	IResource,
	Image,
	ImageTexture,
};

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


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// Data resulting from loading a sprite_#.bin file.
pub struct BinSprite {
	base: Base<Resource>,
	/// A [PackedByteArray] representing the raw pixel vector.
	#[export]
	pub pixels: PackedByteArray,
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
			base: base,
			pixels: PackedByteArray::from(vec![]),
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
	pub fn new_from_data(pixels: PackedByteArray, image: Gd<Image>, bit_depth: u16, palette: PackedByteArray) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base: base,
				pixels: pixels,
				image: Some(image.clone()),
				texture: Some(ImageTexture::create_from_image(image).unwrap()),
				bit_depth: bit_depth,
				palette: palette,
			}
		});
	}
}