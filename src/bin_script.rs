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
	/// Display name for this instruction.
	#[export] pub display_name: GString,
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
			display_name: GString::from(""),
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
		
		if self.size == 1 {
			bin_data.push(self.value as u8);
		}
		
		else {
			bin_data.extend(self.value.to_le_bytes());
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


	pub fn from_data(id: u8, display_name: GString, arguments: Array<Gd<InstructionArgument>>) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				id,
				display_name,
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
	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		bin_data.extend(self.variables.to_vec());

		for action in self.actions.iter_shared() {
			let item = action.bind();
			bin_data.extend(item.to_bin());
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