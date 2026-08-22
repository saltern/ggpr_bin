// Based off https://github.com/gdkchan/GGXrdRevelatorDec
// Which itself seems based off https://gist.github.com/AltimorTASDK/be1f7369af0b02c3816b

use std::fs;
use std::path::PathBuf;

use godot::prelude::*;

const ENCRYPTED_SIGNATURE: u32 = 0x41534743;

const MERSENNE_LENGTH: usize = 624;
const MERSENNE_INIT: u32 = 0x6C078965;

const MERSENNE_TWIST1: u32 = 0x80000000;
const MERSENNE_TWIST2: u32 = 0x7FFFFFFF;
const MERSENNE_TWIST3: u32 = 0x9908B0DF;

const MERSENNE_PRNG1: u32 = 0x9D2C5680;
const MERSENNE_PRNG2: u32 = 0xEFC60000;

const LAST_OUT: u32 = 0x43415046;


struct MersenneTwister {
	index: usize,
	table: Vec<u32>,
}


impl MersenneTwister {
	pub fn initialize(&mut self, seed: u32) {
		self.table.push(seed);

		for i in 1..MERSENNE_LENGTH {
			let previous: u32 = self.table[i - 1];
			self.table.push(MERSENNE_INIT * (previous ^ (previous >> 30)) + i as u32);
		}

		self.index = MERSENNE_LENGTH;
	}

	fn twist(&mut self) {
		for i in 0..MERSENNE_LENGTH {
			let value: u32 =
				(self.table[i] & MERSENNE_TWIST1) +
				(self.table[(i + 1) % MERSENNE_LENGTH] & MERSENNE_TWIST2);

			self.table[i] = self.table[(i + 397) % MERSENNE_LENGTH] ^ (value >> 1);

			if (value & 1) != 0 {
				self.table[i] ^= MERSENNE_TWIST3;
			}

			self.index = 0;
		}
	}

	pub fn get_next_number(&mut self) -> u32 {
		if self.index >= MERSENNE_LENGTH {
			self.twist();
		}
		
		let mut value: u32 = self.table[self.index];

		value ^= value >> 11;
		value ^= (value << 7) & MERSENNE_PRNG1;
		value ^= (value << 15) & MERSENNE_PRNG2;
		value ^= value >> 18;

		self.index += 1;

		return value;
	}
}


#[derive(GodotClass)]
#[class(tool, base=RefCounted, no_init)]
struct BinDecrypter {}


#[godot_api]
impl BinDecrypter {
	/// Decrypts an entire directory of .bin files, overwriting
	/// each file's contents with the decrypted data.
	#[func] pub fn decrypt_folder(path: String) {
		let path_buf: PathBuf = PathBuf::from(&path);

		for result_entry in path_buf.read_dir().unwrap() {
			// Error, directory: skip
			if result_entry.is_err() {
				continue;
			}

			let entry = result_entry.unwrap();

			match entry.file_type() {
				Ok(file_type) => if file_type.is_dir() {
					continue;
				}

				_ => continue,
			}

			// Attempt read
			match fs::read(entry.path()) {
				Ok(data) => {
					let file_name: String;

					match entry.path().file_name() {
						Some(name) => file_name = name.to_str().unwrap().to_uppercase(),
						_ => continue,
					}

					let bin_data = Self::decrypt_file(file_name, data);
					let _ = fs::write(entry.path(), bin_data);
				}

				_ => continue,
			}
		}
	}


	/// Decrypts a [PackedByteArray], returning another containing the processed data.
	/// Returns the data as-is if the encryption signature isn't found.
	#[func] pub fn decrypt_file(file_name: String, bin_data: Vec<u8>) -> Vec<u8> {
		// Do nothing if the file isn't encrypted
		if u32::from_le_bytes([
			bin_data[bin_data.len() - 0x01],
			bin_data[bin_data.len() - 0x02],
			bin_data[bin_data.len() - 0x03],
			bin_data[bin_data.len() - 0x04],
		]) != ENCRYPTED_SIGNATURE {
			return bin_data;
		}

		let name_bytes: Vec<u8> = file_name.to_uppercase().into_bytes();

		let mut seed: u32 = 0;
		for char in 0..name_bytes.len() {
			seed *= 137;
			seed += name_bytes[char] as u32;
		}

		let mut twister = MersenneTwister {
			index: 0,
			table: Vec::with_capacity(MERSENNE_LENGTH),
		};

		twister.initialize(seed);

		let mut last_out: u32 = LAST_OUT;
		let mut cursor: usize = 0;
		let mut output: Vec<u8> = Vec::new();

		while cursor + 4 <= bin_data.len() {
			let value_in: u32 = u32::from_le_bytes([
				bin_data[cursor + 0x00],
				bin_data[cursor + 0x01],
				bin_data[cursor + 0x02],
				bin_data[cursor + 0x03],
			]);

			last_out ^= value_in ^ twister.get_next_number();
			output.extend(last_out.to_le_bytes());
			cursor += 4;
		}

		return output;
	}
}