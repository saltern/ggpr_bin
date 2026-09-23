use std::io::Cursor;
use std::collections::VecDeque;
use std::cmp::min;
use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use godot::prelude::*;
use crate::sprite_transform;

const DEPTH_4			: u16 = 0x0004;
const DEPTH_8			: u16 = 0x0008;

#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
pub struct SpriteCompression {}


#[godot_api]
impl SpriteCompression {
	#[func]
	pub fn compress_acpr(input_pixels: PackedByteArray, bit_depth: u16) -> PackedByteArray {
		const WINDOW_SIZE: usize = 512;
		const TOKEN_SIZE_MAX: usize = 130;

		let pixels: Vec<u8>;

		match bit_depth {
			DEPTH_4 => pixels = sprite_transform::bpp_to_4(input_pixels.to_vec(), true),
			_ => pixels = input_pixels.to_vec(),
		}

		// Loop variables
		let mut current_pixel: usize = 0;
		let mut iterations: u32 = 0;

		// Output bit stream
		let mut compressed_stream: Vec<u8> = Vec::new();
		let mut bit_writer = BitWriter::endian(&mut compressed_stream, BigEndian);

		// Iterate vector
		while current_pixel < pixels.len() {
			// Token window origin point
			let window_origin: usize;

			if current_pixel > WINDOW_SIZE {
				window_origin = current_pixel - WINDOW_SIZE;
			} else {
				window_origin = 0;
			}

			if current_pixel >= 4 && pixels.len() - current_pixel > 2 {
				let mut best_sequence_offset: usize = 0;
				let mut best_sequence_length: usize = 0;
				let mut token_size_max_local: usize = min(TOKEN_SIZE_MAX, current_pixel);
				token_size_max_local = min(token_size_max_local, pixels.len() - current_pixel);

				// New window scan, slower, better compression (matches game's)
				for window_offset in 0..510 {
					let mut sequence_length: usize = 0;

					while sequence_length < token_size_max_local {
						let window_index: usize = window_origin + window_offset + sequence_length;

						if window_index >= current_pixel {
							break;
						}

						if pixels[current_pixel + sequence_length] == pixels[window_index] {
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
			let _ = bit_writer.write(8, pixels[current_pixel]);

			if current_pixel + 1 < pixels.len() {
				let _ = bit_writer.write(8, pixels[current_pixel + 1]);
			} else {
				let _ = bit_writer.write(8, 0u8);
			}

			// Increment position
			current_pixel += 2;
			iterations += 1;
		}

		// Pad and close bit stream
		bit_writer.byte_align().expect(
			"main::make_compressed_sprite() error: Could not align bitstream");
		bit_writer.into_writer();

		let file_byte_length: usize = compressed_stream.len() + 20;

		if file_byte_length % 16 != 0 {
			for _i in 0..(16 - file_byte_length % 16) {
				compressed_stream.push(0xFF);
			}
		}

		let mut bin_data: Vec<u8> = Vec::new();

		// Write iterations
		bin_data.extend([
			(iterations >> 16) as u8,
			(iterations >> 24) as u8,
			(iterations >> 00) as u8,
			(iterations >> 08) as u8,
		]);

		// Write compressed data
		for byte in 0..compressed_stream.len() / 2 {
			bin_data.extend([
				compressed_stream[2 * byte + 1],
				compressed_stream[2 * byte + 0],
			])
		}

		return bin_data.into();
	}


	// DECOMPRESSION ===================================================================================


	// Mode 5 auxiliaries
	fn extract_bits(chunk: &[u8]) -> VecDeque<bool> {
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


	fn pop_bits(chunk: &mut VecDeque<bool>, bit_count: usize) -> u8 {
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


	// Mode 1
	#[func]
	fn decompress_acpr(bin_data: Vec<u8>) -> Vec<u8> {
		const ADDRESS_CLUT			: usize = 0x02;
		const ADDRESS_DEPTH			: usize = 0x04;
		const ADDRESS_WIDTH			: usize = 0x06;
		const ADDRESS_HEIGHT		: usize = 0x08;

		const COLOR_COUNT_4_HALF	: usize = 8;
		const COLOR_COUNT_4_FULL	: usize = 16;
		const COLOR_COUNT_8_HALF	: usize = 128;
		const COLOR_COUNT_8_FULL	: usize = 256;

		const CLUT_NONE				: u16 = 0x0000;
		const CLUT_HALF				: u16 = 0x0010;
		const CLUT_FULL				: u16 = 0x0020;

		const CLUT_SIZE_4_HALF		: usize = 4 * COLOR_COUNT_4_HALF;
		const CLUT_SIZE_4_FULL		: usize = 4 * COLOR_COUNT_4_FULL;
		const CLUT_SIZE_8_HALF		: usize = 4 * COLOR_COUNT_8_HALF;
		const CLUT_SIZE_8_FULL		: usize = 4 * COLOR_COUNT_8_FULL;

		const ADDRESS_HEADER_END	: usize = 0x10;
		
		let bit_depth: u16 = u16::from_le_bytes([
			bin_data[ADDRESS_DEPTH + 0],
			bin_data[ADDRESS_DEPTH + 1],
		]);

		let width: usize = u16::from_le_bytes([
			bin_data[ADDRESS_WIDTH + 0],
			bin_data[ADDRESS_WIDTH + 1],
		]) as usize;

		let height: usize = u16::from_le_bytes([
			bin_data[ADDRESS_HEIGHT + 0],
			bin_data[ADDRESS_HEIGHT + 1],
		]) as usize;

		let clut: u16 = u16::from_le_bytes([
			bin_data[ADDRESS_CLUT + 0],
			bin_data[ADDRESS_CLUT + 1],
		]);

		let pal_size: usize;

		match clut {
			CLUT_NONE => pal_size = 0,

			CLUT_HALF => {
				if bit_depth == DEPTH_4 {
					pal_size = CLUT_SIZE_4_HALF;
				} else {
					pal_size = CLUT_SIZE_8_HALF;
				}
			},

			CLUT_FULL => {
				if bit_depth == DEPTH_4 {
					pal_size = CLUT_SIZE_4_FULL;
				} else {
					pal_size = CLUT_SIZE_8_FULL;
				}
			},

			_ => panic!("bin_sprite.rs::decompress_acpr() -> Invalid CLUT!"),
		}

		let pixel_count: usize = width * height;
		let mut pointer: usize = ADDRESS_HEADER_END + pal_size;

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
		match bit_depth {
			4 => pixel_vector = sprite_transform::bpp_from_4(pixel_vector, true),
			8 => (),
			// Shouldn't ever happen
			_ => panic!("sprite_compress::decompress() error: Invalid BIN bit depth"),
		}

		pixel_vector.resize(width * height, 0u8);
		return pixel_vector;
	}


	// Mode 5
	#[func]
	fn decompress_mode5(bin_data: Vec<u8>) -> Vec<u8> {
		// Read secondary header
		let width: usize = u16::from_le_bytes([
			bin_data[0x0],
			bin_data[0x1],
		]) as usize;

		let mut height: usize = u16::from_le_bytes([
			bin_data[0x2],
			bin_data[0x3],
		]) as usize;

		// 0x14, 0x15: bit depth

		// There are apparently modes 5-4 and 5-5
		let mode: u16 = u16::from_le_bytes([
			bin_data[0x6],
			bin_data[0x7],
		]);

		// Data chunks
		let from_a: usize = u16::from_le_bytes([
			bin_data[0x8],
			bin_data[0x9],
		]) as usize * 0x08;

		let from_b: usize = u16::from_le_bytes([
			bin_data[0xA],
			bin_data[0xB],
		]) as usize * 0x08;

		let from_c: usize = u16::from_le_bytes([
			bin_data[0xC],
			bin_data[0xD],
		]) as usize * 0x08;

		let from_d: usize = u16::from_le_bytes([
			bin_data[0xE],
			bin_data[0xF],
		]) as usize * 0x08;

		let mut chunk_a: VecDeque<bool> = Self::extract_bits(&bin_data[from_a..from_b]);
		let mut chunk_b: VecDeque<bool> = Self::extract_bits(&bin_data[from_b..from_c]);
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
								pixel_a = Self::pop_bits(&mut chunk_b, 4);
								cache_1 = Self::pop_bits(&mut chunk_b, 4);
								pixel_c = Self::pop_bits(&mut chunk_b, 4);
								cache_2 = Self::pop_bits(&mut chunk_b, 4);

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
								cache_1 = Self::pop_bits(&mut chunk_b, 4);
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
								cache_1 = Self::pop_bits(&mut chunk_b, 4);
							}
							if chunk_a.pop_front().unwrap() {
								cache_2 = Self::pop_bits(&mut chunk_b, 4);
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

		return pixel_vector;
	}


	// Modes 19 and 20 (0x13 and 0x14)
	#[func]
	fn decompress_ggx(bin_data: Vec<u8>) -> Vec<u8> {
		const ADDRESS_MODE			: usize = 0x00;
		const ADDRESS_CLUT			: usize = 0x01;
		const ADDRESS_WIDTH			: usize = 0x02;
		const ADDRESS_HEIGHT		: usize = 0x04;
		
		const MODE_8_BPP			: u8 = 0x13;
		const MODE_4_BPP			: u8 = 0x14;
		
		//const CLUT_NONE				: u8 = 0x0F;
		const CLUT_HALF				: u8 = 0x02;
		const CLUT_FULL				: u8 = 0x00;
		
		let width: usize = u16::from_le_bytes([
			bin_data[ADDRESS_WIDTH + 0],
			bin_data[ADDRESS_WIDTH + 1],
		]) as usize;

		let height: usize = u16::from_le_bytes([
			bin_data[ADDRESS_HEIGHT + 0],
			bin_data[ADDRESS_HEIGHT + 1],
		]) as usize;

		let bit_depth: u16;

		match bin_data[ADDRESS_MODE] {
			MODE_4_BPP => bit_depth = DEPTH_4,
			MODE_8_BPP => bit_depth = DEPTH_8,
			_ => panic!("bin_sprite.rs::decompress_ggx() -> Invalid bit depth!"),
		}

		let mut pointer: usize = 0x10;
		let clut_size: usize = (4 * 2u8.pow(bit_depth as u32)) as usize;
		
		match bin_data[ADDRESS_CLUT] & 0xF {
			CLUT_HALF => pointer += clut_size / 2,
			CLUT_FULL => pointer += clut_size,
			_ => (),
		}
		
		let mut pixel_vector: Vec<u8> = Vec::new();

		while pixel_vector.len() < width * height {
			// Literals
			if bin_data[pointer] & 0xC0 == 0 {
				for _i in 0..bin_data[pointer] as usize + 1 {
					pointer += 0x01;

					match bit_depth {
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
				if bit_depth == DEPTH_4 {
					token_count *= 2;
				}

				for _i in 0..token_count {
					pixel_vector.push(pixel_vector[pixel_vector.len() - 1]);
				}
			}

			// Next byte
			pointer += 0x01;
		}
		return pixel_vector;
	}
}