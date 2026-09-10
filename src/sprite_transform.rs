use std::cmp;
use godot::prelude::*;


#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
/// Link class to access Rust sprite transformation functions from GDScript.
struct SpriteTransformer {}


#[godot_api]
impl SpriteTransformer {
	/// Reindex the given array of pixels.
	#[func] pub fn transform_pixels(vector: Vec<u8>) -> Vec<u8> {
		return reindex_vector(vector);
	}
	
	
	/// Reindex the given palette. Its size should be a multiple of 4.
	#[func] pub fn transform_palette(vector: Vec<u8>) -> Vec<u8> {
		return reindex_rgba_vector(vector);
	}
	
	
	/// Flip the given array of pixels horizontally.
	#[func] pub fn flip_horz(vector: Vec<u8>, width: i64) -> Vec<u8> {
		let w: usize = width as usize;
		let h: usize = vector.len() / w;
		return flip_h(vector, w, h);
	}
	
	
	/// Flip the given array of pixels vertically.
	#[func] pub fn flip_vert(vector: Vec<u8>, width: i64) -> Vec<u8> {
		let w: usize = width as usize;
		let h: usize = vector.len() / w;
		return flip_v(vector, w, h);
	}
	
	
	/// Flip the given array of pixels along both axes.
	#[func] pub fn flip_both(vector: Vec<u8>) -> Vec<u8> {
		let mut temp_vec: Vec<u8> = Vec::with_capacity(vector.len());
		
		for byte in 0..vector.len() {
			temp_vec.push(vector[vector.len() - byte - 1]);
		}
		
		return temp_vec;
	}
}


pub fn reindex_vector(vector: Vec<u8>) -> Vec<u8> {
	let mut temp_vec: Vec<u8> = Vec::new();

	for pixel in vector {
		temp_vec.push(transform_index(pixel));
	}
	
	return temp_vec;
}


pub fn reindex_rgba_vector(vector: Vec<u8>) -> Vec<u8> {
	let mut temp_vec: Vec<u8> = vec![0u8; vector.len()];
	
	let color_count: usize = vector.len() / 4;
	
	for index in 0..color_count {
		let new_index = transform_index(index as u8) as usize;
		temp_vec[4 * index + 0] = vector[4 * new_index + 0];
		temp_vec[4 * index + 1] = vector[4 * new_index + 1];
		temp_vec[4 * index + 2] = vector[4 * new_index + 2];
		temp_vec[4 * index + 3] = vector[4 * new_index + 3];
	}
	
	return temp_vec;
}


pub fn transform_index(index: u8) -> u8 {
	// Simpler, possibly faster approach
	match index & 0x18 {
		0x10 => index - 8,
		0x08 => index + 8,
		_ => index,
	}
}


pub fn indexed_as_rgb(input_pixels: Vec<u8>, palette: Vec<u8>) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	for pixel in 0..input_pixels.len() {
		output_pixels.push(palette[4 * input_pixels[pixel] as usize]);
	}
	
	return output_pixels;
}


pub fn bpp_from_1(input_pixels: Vec<u8>) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	for index in 0..input_pixels.len() {
		let mut byte: u8 = input_pixels[index];
		byte = byte.reverse_bits();
		
		for shift in 0..8 {
			output_pixels.push((byte >> shift) & 0x1);
		}
	}
	
	return output_pixels;
}


pub fn bpp_from_2(input_pixels: Vec<u8>) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	for index in 0..input_pixels.len() {
		output_pixels.push((input_pixels[index] >> 6) & 0x3);
		output_pixels.push((input_pixels[index] >> 4) & 0x3);
		output_pixels.push((input_pixels[index] >> 2) & 0x3);
		output_pixels.push((input_pixels[index] >> 0) & 0x3);
	}
	
	return output_pixels;
}


pub fn bpp_from_4(input_pixels: Vec<u8>, flip: bool) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	// When decompressing a 4 bpp sprite, the resulting pixel vector will contain
	// two pixels per byte (0000-0000). Separate and push them to output.
	if flip {
		for index in 0..input_pixels.len() {
			output_pixels.push(input_pixels[index] & 0xF);
			output_pixels.push(input_pixels[index] >> 4);
		}
	}
	
	else {
		for index in 0..input_pixels.len() {
			output_pixels.push(input_pixels[index] >> 4);
			output_pixels.push(input_pixels[index] & 0xF);
		}
	}
	
	return output_pixels;
}


pub fn bpp_to_4(input_pixels: Vec<u8>, flip: bool) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	let mut index: usize = 0;
	
	// When compressing a 4 bpp sprite, two pixels become one byte. Chop two adjacent
	// pixels down to 4 bits and combine them into one 8-bit value.
	while index < input_pixels.len() {
		if index + 1 < input_pixels.len() {
			output_pixels.push(
				cmp::min(input_pixels[index + flip as usize], 0xF) << 4 |
				cmp::min(input_pixels[index + !flip as usize], 0xF) & 0xF
			);
		}
		
		else {
			if flip {
				output_pixels.push(cmp::min(input_pixels[index], 0xF) & 0xF);
			}
			
			else {
				output_pixels.push(cmp::min(input_pixels[index], 0xF) << 4);
			}
		}
		
		index += 2;
	}
	
	return output_pixels;
}


pub fn align_to_4(input_pixels: Vec<u8>, height: usize) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	let width: usize = input_pixels.len() / height;
	let padding: usize = width % 2;
	
	for y in 0..height {
		for x in 0..width {
			output_pixels.push(input_pixels[y * width + x]);
		}
		
		for _x in 0..padding {
			output_pixels.push(0x00);
		}
	}
	
	return output_pixels;
}


pub fn trim_padding(input_pixels: Vec<u8>, width: usize, height: usize, reverse: bool) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	let row_width: usize = input_pixels.len() / height;

	if reverse {
		for y in (0..height).rev() {
			for x in 0..width {
				output_pixels.push(input_pixels[y * row_width + x]);
			}
		}
	}
	
	else {
		for y in 0..height {
			for x in 0..width {
				output_pixels.push(input_pixels[y * row_width + x]);
			}
		}
	}
	
	return output_pixels;
}


pub fn limit_16_colors(input_pixels: Vec<u8>) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	for pixel in 0..input_pixels.len() {
		output_pixels.push(cmp::min(input_pixels[pixel], 0xF));
	}
	
	return output_pixels;
}


pub fn alpha_halve(input_palette: Vec<u8>) -> Vec<u8> {
	let mut palette: Vec<u8> = input_palette.clone();
	let color_count: usize = palette.len() / 4;
	
	for color in 0..color_count {
		let alpha: usize = 4 * color + 3;
		
		if palette[alpha] == 0xFF {
			palette[alpha] = 0x80;
		} else {
			palette[alpha] = palette[alpha] / 2;
		}
	}
	
	return palette;
}


pub fn alpha_double(input_palette: Vec<u8>) -> Vec<u8> {
	let mut palette: Vec<u8> = input_palette.clone();
	let color_count: usize = palette.len() / 4;
	
	for color in 0..color_count {
		let alpha: usize = 4 * color + 3;
		
		if palette[alpha] == 0x80 {
			palette[alpha] = 0xFF;
		} else {
			palette[alpha] = palette[alpha] * 2;
		}
	}
	
	return palette;
}


pub fn flip_h(input_pixels: Vec<u8>, width: usize, height: usize) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();

	for y in 0..height {
		for x in 0..width {
			output_pixels.push(input_pixels[y * width + width - x - 1]);
		}
	}
	
	return output_pixels;
}


pub fn flip_v(input_pixels: Vec<u8>, width: usize, height: usize) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	for y in 0..height {
		let pointer: usize = (height - y - 1) * width;
		output_pixels.extend_from_slice(&input_pixels[pointer..pointer + width]);
	}
	
	return output_pixels;
}