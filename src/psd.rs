use std::io::{Write, BufWriter};
use std::fs::File;
use std::cmp::min;

const HEADER: [u8; 14] = [
	// Signature
	0x38, 0x42, 0x50, 0x53,
	// Version
	0x00, 0x01,
	// Reserved zero bytes
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	// Channel count (3, RGB)
	0x00, 0x03,
];

const COLOR_DATA: [u8; 8] = [
	// Color depth
	0x00, 0x08,
	// Color mode (RGB)
	0x00, 0x03,
	// Data length
	0x00, 0x00, 0x00, 0x00,
];

const IMAGE_RESOURCES: [u8; 4] = [
	// Section length
	0x00, 0x00, 0x00, 0x00,
];


pub fn make_psd(
	image_w: i32, image_h: i32, layers: Vec<(&str, Vec<u8>, (i32, i32, i32, i32))>, path: String
) {
	let file: File;

	match File::create(path) {
		Ok(result) => file = result,
		_ => {
			println!("Could not write .PSD file!");
			return;
		},
	}

	let ref mut buffer = BufWriter::new(file);

	// Static parts of header
	let _ = buffer.write_all(&HEADER);

	// Height, width
	let _ = buffer.write_all((image_h as u32).to_be_bytes().as_slice());
	let _ = buffer.write_all((image_w as u32).to_be_bytes().as_slice());

	// Color data, image resources (also static)
	let _ = buffer.write_all(&COLOR_DATA);
	let _ = buffer.write_all(&IMAGE_RESOURCES);

	// Layer and mask info
	let _ = buffer.write_all(generate_layer_and_mask_info(image_w, image_h, layers).as_slice());

	// Fake composite
	let _ = buffer.write_all(generate_fake_composite(image_w, image_h).as_slice());

	// Finish up
	let _ = buffer.flush();
}


fn generate_layer_and_mask_info(
	w: i32, h: i32, layers: Vec<(&str, Vec<u8>, (i32, i32, i32, i32))>
) -> Vec<u8> {
	let mut layer_and_mask_info: Vec<u8> = Vec::new();

	let layer_info: Vec<u8> = generate_layer_info(w, h, layers);

	layer_and_mask_info.extend((layer_info.len() as u32).to_be_bytes());
	layer_and_mask_info.extend(layer_info);

	return layer_and_mask_info;
}


fn generate_layer_info(
	w: i32, h: i32, layers: Vec<(&str, Vec<u8>, (i32, i32, i32, i32))>
) -> Vec<u8> {
	let mut layer_info: Vec<u8> = Vec::new();
	let mut info_chunk: Vec<u8> = Vec::new();
	let mut layer_records: Vec<u8> = Vec::new();
	let mut pixel_data: Vec<u8> = Vec::new();

	for layer in layers.iter() {
		let channels_data = generate_pixel_data(w, h, &layer.1, layer.2);
		layer_records.extend(generate_layer_record(layer.0, layer.2, channels_data.1));
		pixel_data.extend(channels_data.0);
	}

	info_chunk.extend((layers.len() as u16).to_be_bytes());
	info_chunk.extend(layer_records);
	info_chunk.extend(pixel_data);

	if info_chunk.len() % 4 != 0 {
		for _byte in 0..info_chunk.len() % 4 {
			info_chunk.push(0x00);
		}
	}

	layer_info.extend((info_chunk.len() as u32).to_be_bytes());
	layer_info.extend(info_chunk);

	return layer_info;
}


fn generate_layer_record(
	name: &str, rect: (i32, i32, i32, i32), channel_length: Vec<usize>
) -> Vec<u8> {
	let mut layer_record: Vec<u8> = Vec::new();

	// Boundaries: top, left, down, right
	layer_record.extend(rect.0.to_be_bytes());
	layer_record.extend(rect.2.to_be_bytes());
	layer_record.extend(rect.1.to_be_bytes());
	layer_record.extend(rect.3.to_be_bytes());

	// Channel count
	layer_record.extend(4u16.to_be_bytes());

	// Channel info
	for channel in -1isize..3 {
		layer_record.extend((channel as u16).to_be_bytes());
		let final_length: usize = channel_length[(channel + 1) as usize];
		layer_record.extend((final_length as u32).to_be_bytes());
	}

	layer_record.extend("8BIM".as_bytes());		// Signature
	layer_record.extend("norm".as_bytes());		// Blend mode
	layer_record.push(0xFF);					// Opacity
	layer_record.push(0x00);					// Clipping mode
	layer_record.push(0x00);					// Flags
	layer_record.push(0x00);					// Filler byte, needed

	// Extra data
	let extra_data: Vec<u8> = generate_layer_extra_data(name);
	layer_record.extend((extra_data.len() as u32).to_be_bytes());
	layer_record.extend(extra_data);

	return layer_record;
}


fn generate_layer_extra_data(name: &str) -> Vec<u8> {
	let mut extra_data: Vec<u8> = Vec::new();

	extra_data.extend(00u32.to_be_bytes());		// Mask data length
	extra_data.extend(00u32.to_be_bytes());		// Blending ranges length

	// Layer name
	let name_len: usize = name.len() + 1;

	extra_data.push(name.len() as u8);
	extra_data.extend(name.as_bytes());

	// Pad to 4 byte size
	if name_len % 4 != 0 {
		for _byte in 0..4 - name_len % 4 {
			extra_data.push(0x00);
		}
	}

	return extra_data;
}


fn generate_pixel_data(
	width: i32, height: i32, pixels: &Vec<u8>, rect: (i32, i32, i32, i32)
) -> (Vec<u8>, Vec<usize>) {
	let mut pixel_data: Vec<u8> = Vec::new();

	let channel_order: Vec<usize> = vec![3, 0, 1, 2];
	let mut channel_sizes: Vec<usize> = Vec::with_capacity(4);

	// A, R, G, B
	for channel in channel_order.iter() {
		let mut channel_pixels: Vec<u8> = Vec::with_capacity((width * height) as usize);

		// Build pixel vector for this channel
		for row in rect.0..rect.1 {
			for col in rect.2..rect.3 {
				let pixel = (row * width + col) as usize;
				channel_pixels.push(pixels[(4 * pixel) + channel])
			}
		}

		// Compress and append this channel
		let rle_data = compress_channel(&channel_pixels, rect);
		channel_sizes.push(rle_data.len());
		pixel_data.extend(rle_data);
	}

	return (pixel_data, channel_sizes);
}


fn compress_channel(pixels: &Vec<u8>, rect: (i32, i32, i32, i32)) -> Vec<u8> {
	let mut rle_data: Vec<u8> = Vec::new();
	let mut lengths: Vec<u8> = Vec::new();	// stores be u16s
	let mut packets: Vec<u8> = Vec::new();

	let rect_w = rect.3 - rect.2;
	let rect_h = rect.1 - rect.0;

	// Scan layer rect height
	for row in 0..rect_h {
		let mut row_length: u16 = 0;
		let from = row * rect_w;
		let mut column: i32 = 0;

		while column < rect_w {
			let limit = min(127, rect_w - column);

			// Token scan
			{
				let mut token_length: i32 = 0;
				let at = (from + column) as usize;
				let token_pixel: u8 = pixels[at];

				while pixels[at + token_length as usize] == token_pixel && token_length < limit - 1
				{
					token_length += 1;
				}

				// Register token if at least 2 pixels long.
				if token_length >= 2 {
					let header_byte = -(token_length - 1) as i8;
					packets.push(header_byte as u8);
					packets.push(token_pixel);
					column += token_length;
					row_length += 2;
					continue;
				}
			}

			// Literal scan
			{
				let mut literal_length: i32 = 0;
				let mut match_length: u8 = 0;
				let at: usize = (from + column) as usize;
				let mut last_match: u8 = pixels[at];
				let mut literal: Vec<u8> = Vec::new();

				while match_length < 3 && literal_length < limit {
					let this_pixel: u8 = pixels[at + literal_length as usize];

					if this_pixel == last_match {
						match_length += 1;
					} else {
						match_length = 0;
						last_match = this_pixel;
					}

					literal_length += 1;
					literal.push(this_pixel);
				}

				// Register literal
				packets.push((literal_length - 1) as u8);
				packets.extend(literal);
				column += literal_length;
				row_length += literal_length as u16 + 1;
			}
		}
		lengths.extend(row_length.to_be_bytes());
	}

	rle_data.extend(1u16.to_be_bytes());
	rle_data.extend(lengths);
	rle_data.extend(packets);

	return rle_data;
}


fn generate_fake_composite(width: i32, height: i32) -> Vec<u8> {
	let mut fake_composite: Vec<u8> = Vec::new();

	// Compression mode
	fake_composite.extend(1u16.to_be_bytes());

	let full_tokens = width / 128;
	let remainder = width - (128 * full_tokens);
	let mut row_length: u16 = 2 * full_tokens as u16;

	if remainder > 0 {
		row_length += 2;
	}

	// Counts
	for _scanline in 0..3 * height as usize {
		fake_composite.extend(row_length.to_be_bytes());
	}

	// RLE Data
	for _channel in 0..3 {
		for _scanline in 0..height as usize {
			for _token in 0..full_tokens {
				fake_composite.push(-127i8 as u8);
				fake_composite.push(0x00);
			}

			if remainder == 1 {
				fake_composite.push(0u8);
			} else {
				fake_composite.push(-(remainder - 1) as u8);
			}

			fake_composite.push(0u8);
		}
	}

	return fake_composite;
}