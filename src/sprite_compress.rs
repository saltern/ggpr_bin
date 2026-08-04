use std::io::Cursor;
use std::cmp::min;
use std::collections::VecDeque;
use bitstream_io::{BitReader, BitRead, BitWriter, BitWrite, BigEndian};
use crate::{
	bin_sprite::BinHeader,
	sprite_transform,
};

const WINDOW_SIZE: usize = 512;
const TOKEN_SIZE_MAX: usize = 130;

pub struct CompressedData {
	pub iterations: usize,
	pub stream: Vec<u8>,
}

pub struct SpriteData {
	pub width: u16,
	pub height: u16,
	pub bit_depth: u16,
	pub pixels: Vec<u8>,
	pub pixels_rgba: Vec<u8>,
	pub palette: Vec<u8>,
}

impl Default for SpriteData {
	fn default() -> SpriteData {
		SpriteData {
			width: 0,
			height: 0,
			bit_depth: 8,
			pixels: Vec::new(),
			pixels_rgba: Vec::new(),
			palette: Vec::new(),
		}
	}
}


pub fn get_palette(bin_data: &Vec<u8>, header: &BinHeader) -> Vec<u8> {
	let mut palette: Vec<u8> = Vec::new();
	
	if header.clut != 0x00 {
		// Determine color count
		// Possibilities: 8, 16, 128, 256
		let mut color_count: usize = 2u16.pow(header.bit_depth as u32) as usize;
		if header.clut == 0x10 {
			color_count /= 2;
		}
		
		// Read palette
		for index in 0..color_count {
			// RGBA
			palette.push(bin_data[0x10 + 4 * index + 0]);
			palette.push(bin_data[0x10 + 4 * index + 1]);
			palette.push(bin_data[0x10 + 4 * index + 2]);
			palette.push(bin_data[0x10 + 4 * index + 3]);
		}
	}
	
	return palette;
}

pub fn compress(mut data: SpriteData) -> CompressedData {
	// Bit depth management
	match data.bit_depth {
		4 => data.pixels = sprite_transform::bpp_to_4(data.pixels, true),
		8 => (), // No transform needed
		// Shouldn't ever happen
		_ => panic!("sprite_compress::compress() error: Invalid SpriteData bit depth"),
	}

	// Loop variables
	let mut current_pixel: usize = 0;
	let mut iterations: usize = 0;
	
	// Output bit stream
	let mut compressed_stream: Vec::<u8> = Vec::new();
	let mut bit_writer = BitWriter::endian(&mut compressed_stream, BigEndian);
	
	// Iterate vector
	while current_pixel < data.pixels.len() {
		// Token window origin point
		let window_origin: usize;
		
		if current_pixel > WINDOW_SIZE {
			window_origin = current_pixel - WINDOW_SIZE;
		} else {
			window_origin = 0;
		}
		
		if current_pixel >= 4 && data.pixels.len() - current_pixel > 2 {
			let mut best_sequence_offset: usize = 0;
			let mut best_sequence_length: usize = 0;
			let mut token_size_max_local: usize = min(TOKEN_SIZE_MAX, current_pixel);
			token_size_max_local = min(token_size_max_local, data.pixels.len() - current_pixel);
			
			// New window scan, slower, better compression (matches game's)
			for window_offset in 0..510 {
				let mut sequence_length: usize = 0;
				
				while sequence_length < token_size_max_local {
					let window_index: usize = window_origin + window_offset + sequence_length;
					
					if window_index >= current_pixel {
						break;
					}
						
					if data.pixels[current_pixel + sequence_length] == data.pixels[window_index] {
						sequence_length += 1;
					} else {
						break;
					}
				}
				
				if sequence_length > best_sequence_length {
					best_sequence_length = sequence_length;
					best_sequence_offset = window_offset;
				}
				
				if sequence_length >= token_size_max_local {
					break;
				}
			}
			
			if best_sequence_length > 2 {
				let _ = bit_writer.write_bit(false);
				let _ = bit_writer.write(9, best_sequence_offset as u16);
				let _ = bit_writer.write(7, (best_sequence_length as u8) - 3);
				current_pixel += best_sequence_length;
				iterations += 1;
				continue;
			}
		}
		
		// Literal indicator
		let _ = bit_writer.write_bit(true);
		
		// Pixels
		let _ = bit_writer.write(8, data.pixels[current_pixel]);
		
		if current_pixel + 1 < data.pixels.len() {
			let _ = bit_writer.write(8, data.pixels[current_pixel + 1]);
		} else {
			let _ = bit_writer.write(8, 0u8);
		}
		
		// Increment position
		current_pixel += 2;		
		iterations += 1;
	}
	
	// Pad and close bit stream
	bit_writer.byte_align().expect("main::make_compressed_sprite() error: Could not align bitstream");
	bit_writer.into_writer();
	
	let file_byte_length: usize = compressed_stream.len() + 20;
	
	if file_byte_length % 16 != 0 {
		for _i in 0..(16 - file_byte_length % 16) {
			compressed_stream.push(255);
		}
	}
	
	return CompressedData {
		iterations,
		stream: compressed_stream,
	};
}


pub fn decompress(bin_data: &Vec<u8>, header: BinHeader) -> SpriteData {
	println!("sprite_compress.rs::decompress()");
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
	while pointer + 1 < bin_data.len() {
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
		4 => pixel_vector = sprite_transform::bpp_from_4(pixel_vector, true),
		8 => (),
		// Shouldn't ever happen
		_ => panic!("sprite_compress::decompress() error: Invalid BIN bit depth"),
	}
	
	pixel_vector.resize(header.width as usize * header.height as usize, 0u8);

	return SpriteData {
		width: header.width,
		height: header.height,
		bit_depth: header.bit_depth,
		pixels: pixel_vector,
		pixels_rgba: vec![],
		palette,
	};
}


pub fn decompress_ggx(bin_data: &Vec<u8>, header: BinHeader) -> SpriteData {
	println!("sprite_compress.rs::decompress_ggx()");
	let mut pointer: usize = 0x10;
	let mut palette: Vec<u8> = Vec::new();

	// Get embedded palette
	if header.clut != 0x00 {
		let mut color_count: usize = 2u16.pow(header.bit_depth as u32) as usize;
		if header.clut == 0x10 {
			color_count /= 2;
		}

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

	let mut pixel_vector: Vec<u8> = Vec::new();

	while pixel_vector.len() < header.width as usize * header.height as usize {
		// Literals
		if bin_data[pointer] & 0xC0 == 0 {
			for _i in 0..bin_data[pointer] as usize + 1 {
				pointer += 0x01;

				match header.bit_depth {
					4 => {
						pixel_vector.push(bin_data[pointer] & 0xF);
						pixel_vector.push(bin_data[pointer] >> 4);
					},

					_ => {
						pixel_vector.push(bin_data[pointer]);
					}
				}
			}
		}

		// Tokens
		else {
			let mut token_count: usize = (bin_data[pointer] as usize + 0xC3) & 0xFF;
			if header.bit_depth == 4 {
				token_count *= 2;
			}

			for _i in 0..token_count {
				pixel_vector.push(pixel_vector[pixel_vector.len() - 1]);
			}
		}

		// Next byte
		pointer += 0x01;
	}

	SpriteData {
		width: header.width,
		height: header.height,
		bit_depth: header.bit_depth,
		pixels: pixel_vector,
		pixels_rgba: vec![],
		palette,
	}
}


pub fn extract_bits(chunk: &[u8]) -> VecDeque<bool> {
	let mut new_chunk: VecDeque<bool> = VecDeque::new();
	let mut byte: u8;
	
	for pointer in 0..chunk.len() {
		byte = chunk[pointer];
		for _i in 0..8 {
			new_chunk.push_back((byte & 1) == 1);
			byte >>= 1;
		}
	}
	
	return new_chunk;
}


pub fn pop_bits(chunk: &mut VecDeque<bool>, bit_count: usize) -> u8 {
	let mut byte: u8 = 0;

	for bit in 0..bit_count {
		match chunk.pop_front() {
			Some(true) => byte |= 1 << bit,
			Some(false) => (),
			None => break,
		}
	}

	return byte;
}


pub fn decompress_mode5(bin_data: &Vec<u8>, header: BinHeader) -> SpriteData {
	println!("sprite_compress.rs::decompress_mode5()");
	let palette: Vec<u8> = get_palette(bin_data, &header);
	print!("{:?}", palette);
	let data_offset: usize = 0x10 + palette.len();
	
	// Read secondary header
	let width: usize = u16::from_le_bytes([
		bin_data[data_offset + 0x0],
		bin_data[data_offset + 0x1],
	]) as usize;
	
	let mut height: usize = u16::from_le_bytes([
		bin_data[data_offset + 0x2],
		bin_data[data_offset + 0x3],
	]) as usize;
	
	// 0x14, 0x15: bit depth
	
	// There are apparently modes 5-4 and 5-5
	let mode: u16 = u16::from_le_bytes([
		bin_data[data_offset + 0x6],
		bin_data[data_offset + 0x7],
	]);
	
	// Data chunks
	let from_a: usize = u16::from_le_bytes([
		bin_data[data_offset + 0x8],
		bin_data[data_offset + 0x9],
	]) as usize * 0x08 + data_offset;
	
	let from_b: usize = u16::from_le_bytes([
		bin_data[data_offset + 0xA],
		bin_data[data_offset + 0xB],
	]) as usize * 0x08 + data_offset;
	
	let from_c: usize = u16::from_le_bytes([
		bin_data[data_offset + 0xC],
		bin_data[data_offset + 0xD],
	]) as usize * 0x08 + data_offset;
	
	let from_d: usize = u16::from_le_bytes([
		bin_data[data_offset + 0xE],
		bin_data[data_offset + 0xF],
	]) as usize * 0x08 + data_offset;
	
	let mut chunk_a: VecDeque<bool> = extract_bits(&bin_data[from_a..from_b]);
	let mut chunk_b: VecDeque<bool> = extract_bits(&bin_data[from_b..from_c]);
	let mut chunk_c: VecDeque<u8> = VecDeque::new();
	let mut chunk_d: Vec<u8> = Vec::new(); // 1 byte at a time
	chunk_d.extend_from_slice(&bin_data[from_d..]);

	// 12*5 bits, then skip 4 bits
	let mut chunk_c_raw: Vec<u8> = Vec::new();
	chunk_c_raw.extend_from_slice(&bin_data[from_c..from_d]);

	{	// Save myself some chunk-C-based headache
		for i in 0..chunk_c_raw.len() / 8 {
			let mut qword: u64 = u64::from_le_bytes([
				chunk_c_raw[8 * i + 0], chunk_c_raw[8 * i + 1],
				chunk_c_raw[8 * i + 2], chunk_c_raw[8 * i + 3],
				chunk_c_raw[8 * i + 4], chunk_c_raw[8 * i + 5],
				chunk_c_raw[8 * i + 6], chunk_c_raw[8 * i + 7],
			]);
			
			for _j in 0..12 {
				chunk_c.push_back((qword & 0x1F) as u8);
				qword >>= 5;
			}
		}
	}

	let mut pointer_d: usize = 0;
	
	let pixel_count: usize = width * height;
	let mut pixel_vector: Vec<u8> = Vec::with_capacity(pixel_count);
	pixel_vector.resize(pixel_count, 0);
	
	let mut pointer_write: usize = 0;
	
	height /= 2;

	let mut iterations: u16 = 0;
	let mut cache_1: u8 = 0;
	let mut cache_2: u8 = 0;
	let mut pixel_a: u8 = 0;
	let mut pixel_b: u8 = 0;
	let mut pixel_c: u8 = 0;
	let mut pixel_d: u8 = 0;
	
	if mode == 5 {
		for _y in 0..height {
			for _x in 0..width / 2 {
				if iterations == 0 {
					if chunk_a.pop_front().unwrap() {
						if chunk_a.pop_front().unwrap() {
							// Top line
							pixel_a = chunk_c.pop_front().unwrap();
							cache_1 = chunk_c.pop_front().unwrap();
							pixel_b = cache_1;
							
							// Bottom line
							pixel_c = chunk_c.pop_front().unwrap();
							cache_2 = chunk_c.pop_front().unwrap();
							pixel_d = cache_2;
						}
						
						else if chunk_a.pop_front().unwrap() {
							iterations = chunk_d[pointer_d] as u16 + 3;
							pointer_d += 1;
						}
					}
					
					else {
						if chunk_a.pop_front().unwrap() {
							if chunk_a.pop_front().unwrap() {
								cache_1 = chunk_c.pop_front().unwrap();
								pixel_a = cache_1;
							}
							
							else {
								if chunk_a.pop_front().unwrap() {
									pixel_a = cache_2;
								}
								else {
									pixel_a = cache_1;
								}
							}
							
							pixel_b = pixel_a;
							pixel_c = pixel_a;
							pixel_d = pixel_a;
						}
						
						else {
							if chunk_a.pop_front().unwrap() {
								cache_1 = chunk_c.pop_front().unwrap();
							}
							
							if chunk_a.pop_front().unwrap() {
								cache_2 = chunk_c.pop_front().unwrap();
							}
							
							if chunk_b.pop_front().unwrap() {
								pixel_d = cache_2;
							} else {
								pixel_d = cache_1;
							}
							
							if chunk_b.pop_front().unwrap() {
								pixel_c = cache_2;
							} else {
								pixel_c = cache_1;
							}
							
							if chunk_b.pop_front().unwrap() {
								pixel_b = cache_2;
							} else {
								pixel_b = cache_1;
							}
							
							if chunk_b.pop_front().unwrap() {
								pixel_a = cache_2;
							} else {
								pixel_a = cache_1;
							}
						}
					}
				}
				
				else {
					iterations -= 1;
				}
				
				pixel_vector[pointer_write + 0] = pixel_a;
				pixel_vector[pointer_write + 1] = pixel_b;
				pixel_vector[pointer_write + width + 0] = pixel_c;
				pixel_vector[pointer_write + width + 1] = pixel_d;
				pointer_write += 2;
			}
			pointer_write += width;
		}
	}

	if mode == 4 {
		for _y in 0..height {
			for _x in 0..width / 2 {
				if iterations == 0 {
					if chunk_a.pop_front().unwrap() {
						if chunk_a.pop_front().unwrap() {
							pixel_a = pop_bits(&mut chunk_b, 4);
							cache_1 = pop_bits(&mut chunk_b, 4);
							pixel_c = pop_bits(&mut chunk_b, 4);
							cache_2 = pop_bits(&mut chunk_b, 4);

							pixel_b = cache_1;
							pixel_d = cache_2;
						}

						else if chunk_a.pop_front().unwrap() {
							iterations = chunk_d[pointer_d] as u16 + 3;
							pointer_d += 1;
						}
					}

					else if chunk_a.pop_front().unwrap() {
						if chunk_a.pop_front().unwrap() {
							cache_1 = pop_bits(&mut chunk_b, 4);
							pixel_c = cache_1;
						}

						else if chunk_a.pop_front().unwrap() {
							pixel_c = cache_2;
						} else {
							pixel_c = cache_1;
						}

						pixel_a = pixel_c;
						pixel_b = pixel_c;
						pixel_d = pixel_c;
					}

					else {
						if chunk_a.pop_front().unwrap() {
							cache_1 = pop_bits(&mut chunk_b, 4);
						}
						if chunk_a.pop_front().unwrap() {
							cache_2 = pop_bits(&mut chunk_b, 4);
						}

						if chunk_b.pop_front().unwrap() {
							pixel_d = cache_2;
						} else {
							pixel_d = cache_1;
						}

						if chunk_b.pop_front().unwrap() {
							pixel_c = cache_2;
						} else {
							pixel_c = cache_1;
						}

						if chunk_b.pop_front().unwrap() {
							pixel_b = cache_2;
						} else {
							pixel_b = cache_1;
						}

						if chunk_b.pop_front().unwrap() {
							pixel_a = cache_2;
						} else {
							pixel_a = cache_1;
						}
					}
				}
				
				else {
					iterations -= 1;
				}
				
				pixel_vector[pointer_write + 0] = pixel_a;
				pixel_vector[pointer_write + 1] = pixel_b;
				pixel_vector[pointer_write + width + 0] = pixel_c;
				pixel_vector[pointer_write + width + 1] = pixel_d;
				pointer_write += 2;
			}
			pointer_write += width;
		}
	}

	return SpriteData {
		width: header.width,
		height: header.height,
		bit_depth: header.bit_depth,
		pixels: pixel_vector,
		pixels_rgba: vec![],
		palette,
	}
}