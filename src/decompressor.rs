use std::io::Cursor;
use bitstream_io::{BitReader, BitRead, BigEndian};
use crate::{
	bin_sprite::BinHeader,
	sprite_transform,
};


pub struct SpriteData {
	pub width: u16,
	pub height: u16,
	pub pixels: Vec<u8>,
	pub palette: Vec<u8>,
}

impl Default for SpriteData {
	fn default() -> SpriteData {
		SpriteData {
			width: 0,
			height: 0,
			pixels: Vec::new(),
			palette: Vec::new(),
		}
	}
}


pub fn decompress(bin_data: Vec<u8>, header: BinHeader) -> SpriteData {
	let pixel_count: usize = header.width as usize * header.height as usize;
	let mut pointer: usize = 0x10;
	let mut palette: Vec<u8> = Vec::new();
	
	// Get embedded palette
	if header.clut == 0x20 {
		let color_count: usize = 2u16.pow(header.bit_depth as u32) as usize;
		
		// Get palette
		for index in 0..color_count {
			// RGBA
			palette.push(bin_data[pointer + 4 * index + 0]);
			palette.push(bin_data[pointer + 4 * index + 1]);
			palette.push(bin_data[pointer + 4 * index + 2]);
			palette.push(bin_data[pointer + 4 * index + 3]);
		}
		
		pointer += color_count * 4;
	}
	
	// Read iterations
	let iterations: u32 = u32::from_le_bytes([
		bin_data[pointer + 0x02],
		bin_data[pointer + 0x03],
		bin_data[pointer + 0x00],
		bin_data[pointer + 0x01]
	]);
	
	// Move pointer past iterations
	pointer += 0x04;
	
	// Get byte data
	let mut byte_data: Vec<u8> = Vec::with_capacity(bin_data.len() - pointer);
	while pointer < bin_data.len() {
		byte_data.push(bin_data[pointer + 1]);
		byte_data.push(bin_data[pointer]);
		pointer += 2;
	}
	
	// Read as bit stream
	let mut bit_reader = BitReader::endian(Cursor::new(&byte_data), BigEndian);
	
	// Pixel vector
	let mut pixel_vector: Vec<u8> = Vec::new();
	
	for _i in 0..iterations {
		// Literal mode
		if bit_reader.read_bit().unwrap() == true {
			pixel_vector.push(bit_reader.read(8).unwrap());
			
			// Stray byte guard rail
			if pixel_vector.len() + 1 < pixel_count {
				pixel_vector.push(bit_reader.read(8).unwrap());
			}
		}
		
		// Token mode
		else {			
			let mut window_origin: usize = 0;
			if pixel_vector.len() > 512 {
				window_origin = pixel_vector.len() - 512;
			}
			
			let offset: usize = bit_reader.read::<u16>(9).unwrap() as usize;
			let length: usize = 3 + bit_reader.read::<u8>(7).unwrap() as usize;
			
			for pixel in 0..length {
				pixel_vector.push(pixel_vector[window_origin + offset + pixel]);
			}
		}
	}
	
	// Bit depth management
	match header.bit_depth {
		4 => pixel_vector = sprite_transform::bpp_from_4(pixel_vector),//, true),
		8 => (), // No transform needed
		// Shouldn't ever happen
		_ => panic!("sprite_compress::decompress() error: Invalid BIN bit depth"),
	}
	
	pixel_vector.resize(header.width as usize * header.height as usize, 0u8);
	
	return SpriteData {
		width: header.width,
		height: header.height,
		pixels: pixel_vector,
		palette: palette,
	};
}