pub fn transform_index(mut value: u8) -> u8 {
	// Divide the currently read byte by 8.
	// - If remainder + 2 can be evenly divided by 4, output is byte value - 8
	// - If remainder + 3 can be evenly divided by 4, output is byte value + 8
	// The original value is passed through otherwise
	
	if ((value / 8) + 2) % 4 == 0 {
		value -= 8;
	}
	
	else if ((value / 8) + 3) % 4 == 0 {
		value += 8
	}
	
	return value;
}


pub fn bpp_from_4(input_pixels: Vec<u8>) -> Vec<u8> {//, flip: bool) -> Vec<u8> {
	let mut output_pixels: Vec<u8> = Vec::new();
	
	// When decompressing a 4 bpp sprite, the resulting pixel vector will contain
	// two pixels per byte (0000-0000). Separate and push them to output.
	// if flip {
		// for index in 0..input_pixels.len() {
			// output_pixels.push(input_pixels[index] & 0xF);
			// output_pixels.push(input_pixels[index] >> 4);
		// }
	// }
	
	// else {
	for index in 0..input_pixels.len() {
		output_pixels.push(input_pixels[index] >> 4);
		output_pixels.push(input_pixels[index] & 0xF);
	}
	// }
	
	return output_pixels;
}