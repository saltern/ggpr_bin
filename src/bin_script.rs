use godot::prelude::*;


// Class definitions

#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// An individual argument for BinScriptAction Instructions.
pub struct InstructionArgument {
	base: Base<Resource>,
	/// Size of this argument. Possible values: 1, 2.
	#[export] pub size: u8,
	/// Value of this argument. 8, 16, or 32 bits.
	#[export] pub value: i64,
	/// Whether the value should be signed or not.
	#[export] pub signed: bool,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// An individual instruction in a BinScriptAction.
pub struct Instruction {
	base: Base<Resource>,
	/// ID number of this instruction. 0 - 255.
	#[export] pub id: u8,
	/// List of arguments in this instruction.
	#[export] pub arguments: Array<Gd<InstructionArgument>>,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// An individual script action.
pub struct ScriptAction {
	base: Base<Resource>,
	/// Header: flags. 4 bytes.
	#[export] pub flags: u32,
	/// Header: lvflag. 2 bytes.
	#[export] pub lvflag: u16,
	/// Header: damage. 1 byte.
	#[export] pub damage: u8,
	/// Header: flag2. 1 byte.
	#[export] pub flag2: u8,
	/// Instructions for this action.
	#[export] pub instructions: Array<Gd<Instruction>>,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// A scriptable object's script. Contains all of its actions.
pub struct BinScript {
	base: Base<Resource>,
	/// play_data variables.
	#[export] pub variables: PackedByteArray,
	/// List of script actions.
	#[export] pub actions: Array<Gd<ScriptAction>>,
}


// Trait implementations

#[godot_api] impl IResource for InstructionArgument {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			size: 1,
			value: 0,
			signed: false,
		}
	}
}


#[godot_api] impl IResource for Instruction {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			id: 0,
			arguments: array![],
		}
	}
}


#[godot_api] impl IResource for ScriptAction {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			flags: 0,
			lvflag: 0,
			damage: 0,
			flag2: 0,
			instructions: array![],
		}
	}
}


#[godot_api] impl IResource for BinScript {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			variables: PackedByteArray::new(),
			actions: array![],
		}
	}
}


// Class implementations

#[godot_api] impl InstructionArgument {
	/// Returns a binary representation of this argument.
	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		match self.size {
			1 => bin_data.push(self.value as u8),
			2 => bin_data.extend(u16::to_le_bytes(self.value as u16)),
			_ => bin_data.extend(self.value.to_le_bytes()),
		}
		
		return bin_data;
	}

	pub fn from_data(size: u8, value: i64, signed: bool) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				size,
				value,
				signed,
			}
		})
	}
}


#[godot_api] impl Instruction {
	/// Returns a binary representation of this instruction.
	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();
		
		bin_data.push(self.id);
		
		for argument in self.arguments.iter_shared() {
			let item = argument.bind();
			bin_data.extend(item.to_bin());
		}
		
		return bin_data;
	}


	pub fn from_data(id: u8, arguments: Array<Gd<InstructionArgument>>) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				id,
				arguments
			}
		});
	}


	/*
	BinScript
		ScriptAction
			Instruction
				InstructionArgument
					-> size
					-> value
				InstructionArgument
					...
	 */
}


#[godot_api] impl ScriptAction {
	/// Returns a binary representation of this action.
	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();
		
		bin_data.extend(self.flags.to_le_bytes());
		bin_data.extend(self.lvflag.to_le_bytes());
		bin_data.push(self.damage);
		bin_data.push(self.flag2);
		
		for instruction in self.instructions.iter_shared() {
			let item = instruction.bind();
			bin_data.extend(item.to_bin());
		}
		
		return bin_data;
	}

	pub fn from_data(
		flags: u32, lvflag: u16, damage: u8, flag2: u8, instructions: Array<Gd<Instruction>>
	) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				flags,
				lvflag,
				damage,
				flag2,
				instructions,
			}
		})
	}
}


#[godot_api] impl BinScript {
	pub fn from_bin(bin_data: Vec<u8>, play_data: bool, instruction_db: &Dictionary) -> Gd<Self> {
		let mut cursor: usize = 0x00;

		if play_data {
			if bin_data[0x01] < 0x81 && bin_data[0x01] > 0x02 {
				if bin_data[0x01] == 0x05 {
					cursor = 0x300
				} else {
					cursor = 0x100;

					if bin_data[0x50] & 0x01 > 0 {
						cursor = 0x180
					}

					if bin_data[0x50] & 0x02 > 0 {
						cursor += 0x80
					}

					if bin_data[0x50] & 0x04 > 0 {
						cursor += 0x80
					}

					if bin_data[0x50] & 0x08 > 0 {
						cursor += 0x80
					}
				}
			} else {
				cursor = 0x80
			}

			// Isuka hack, will fail if first byte in first action's flags is E5 (usually shouldn't)
			if bin_data[cursor] == 0xE5 {
				cursor *= 2
			}
		}

		let variables = PackedByteArray::from(bin_data[0x00..cursor].to_vec());
		let mut actions: Array<Gd<ScriptAction>> = Array::new();

		while cursor < bin_data.len() {
			if bin_data[cursor..cursor + 0x02] == [0xFD, 0x00] {
				break
			}

			let flags = u32::from_le_bytes(bin_data[cursor..cursor + 0x04].try_into().unwrap());
			cursor += 0x04;
			let lvflag = u16::from_le_bytes(bin_data[cursor..cursor + 0x02].try_into().unwrap());
			cursor += 0x02;
			let damage = bin_data[cursor];
			cursor += 0x01;
			let flag2 = bin_data[cursor];
			cursor += 0x01;

			let mut action_over: bool = false;
			let mut instructions: Array<Gd<Instruction>> = Array::new();

			while !action_over && cursor < bin_data.len() {
				let id = bin_data[cursor];
				let entry: Dictionary = instruction_db.at(id).to();
				cursor += 0x01;

				// Add arguments
				let mut arguments: Array<Gd<InstructionArgument>> = Array::new();
				let entry_args: Array<Dictionary> = entry.at("arguments").to();

				for entry_arg in entry_args.iter_shared() {
					let arg_size: u8 = entry_arg.at("size").to();
					let arg_signed: bool = entry_arg.at("signed").to();
					let arg_value: i64;

					if arg_signed {
						match arg_size {
							1 => arg_value = (bin_data[cursor] as i8) as i64,
							2 => arg_value = i16::from_le_bytes(
								bin_data[cursor..cursor + 0x02].try_into().unwrap(),
							) as i64,
							_ => arg_value = i32::from_le_bytes(
								bin_data[cursor..cursor + 0x04].try_into().unwrap()
							) as i64,
						}
					} else {
						match arg_size {
							1 => arg_value = bin_data[cursor] as i64,
							2 => arg_value = u16::from_le_bytes(
								bin_data[cursor..cursor + 0x02].try_into().unwrap()
							) as i64,
							_ => arg_value = u32::from_le_bytes(
								bin_data[cursor..cursor + 0x04].try_into().unwrap()
							) as i64,
						}
					}

					cursor += arg_size as usize;
					arguments.push(
						&InstructionArgument::from_data(arg_size, arg_value, arg_signed)
					);
				}

				instructions.push(&Instruction::from_data(id, arguments));
				action_over = id == 0xFF;
			}

			actions.push(&ScriptAction::from_data(flags, lvflag, damage, flag2, instructions));
		}

		return Gd::from_init_fn(|base| {
			Self {
				base,
				variables,
				actions,
			}
		})
	}

	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		bin_data.extend(self.variables.to_vec());

		for action in self.actions.iter_shared() {
			let item = action.bind();
			bin_data.extend(item.to_bin());
		}

		// End script
		bin_data.extend([0xFD, 0x00]);
		if bin_data.len() % 0x10 != 0 {
			bin_data.resize(bin_data.len() + 0x10 - bin_data.len() % 0x10, 0u8);
		}

		return bin_data;
	}

	pub fn from_data(variables: PackedByteArray, actions: Array<Gd<ScriptAction>>) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				variables,
				actions,
			}
		})
	}
}