use std::cmp::max;
use godot::classes::{animation, Animation};
use godot::prelude::*;

// I don't like this either.
const ID_CELLBEGIN		: u8 = 0x00;
const ID_SEMITRANS		: u8 = 0x06;
const ID_SCALE			: u8 = 0x07;
const ID_ROT			: u8 = 0x08;
const ID_DRAW_NORMAL	: u8 = 0x10;
const ID_DRAW_REVERSE	: u8 = 0x11;
const ID_CELL_JUMP		: u8 = 0x27;
const ID_VISUAL			: u8 = 0x45;
const ID_END_ACTION		: u8 = 0xFF;


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
/// A character's PlayData variables.
pub struct PlayVariables {
	base: Base<Resource>,
	#[export] pub header: u16,
	#[export] pub walk_fwd_x_speed: i16,
	#[export] pub walk_bwd_x_speed: i16,
	#[export] pub dash_x_speed: i16,
	#[export] pub backdash_x_speed: i16,
	#[export] pub backdash_y_speed: i16,
	#[export] pub backdash_gravity: i16,
	#[export] pub jump_fwd_x_speed: i16,
	#[export] pub jump_bwd_x_speed: i16,
	#[export] pub jump_y_speed: i16,
	#[export] pub jump_gravity: i16,
	#[export] pub sjump_fwd_x_speed: i16,
	#[export] pub sjump_bwd_x_speed: i16,
	#[export] pub sjump_y_speed: i16,
	#[export] pub sjump_gravity: i16,
	#[export] pub dash_acceleration: i16,
	#[export] pub dash_resist: i16,
	#[export] pub homing_jump_y_max: i16,
	#[export] pub homing_jump_x_max: i16,
	#[export] pub homing_jump_x_resist: i16,
	#[export] pub homing_target_y_offset: i16,
	#[export] pub airdash_height: i16,
	#[export] pub pd_airdash_fwd_time: i16,
	#[export] pub pd_airdash_bwd_time: i16,
	#[export] pub pd_faint_point: i16,
	#[export] pub pd_defense_point: i16,
	#[export] pub pd_konjo: i16,
	#[export] pub pd_kaishin: i16,
	#[export] pub pd_defense_gravity: i16,
	#[export] pub pd_airdash_count: i16,
	#[export] pub pd_jump_count: i16,
	#[export] pub pd_airdash_fwd_atk_time: i16,
	#[export] pub pd_airdash_bwd_atk_time: i16,
	#[export] pub pd_tension_walk: i16,
	#[export] pub pd_tension_jump: i16,
	#[export] pub pd_tension_dash: i16,
	#[export] pub pd_tension_airdash: i16,
	#[export] pub pdgc_gauge_def_point: i16,
	#[export] pub pdgc_gauge_recovery: i16,
	#[export] pub pd_tension_ib: i16,
	#[export] pub padding: PackedByteArray,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// A character's chain table.
pub struct ChainTable {
	base: Base<Resource>,
	#[export] pub chain_5p: u32,
	#[export] pub chain_6p: u32,
	#[export] pub chain_5k: u32,
	#[export] pub chain_fs: u32,
	#[export] pub chain_cs: u32,
	#[export] pub chain_5h: u32,
	#[export] pub chain_6h: u32,
	#[export] pub chain_2p: u32,
	#[export] pub chain_2k: u32,
	#[export] pub chain_2d: u32,
	#[export] pub chain_2s: u32,
	#[export] pub chain_2h: u32,
	#[export] pub chain_jp: u32,
	#[export] pub chain_jk: u32,
	#[export] pub chain_js: u32,
	#[export] pub chain_jh: u32,
	#[export] pub chain_unk16: u32,
	#[export] pub chain_j2k: u32,
	#[export] pub chain_3p: u32,
	#[export] pub chain_unk19: u32,
	#[export] pub chain_unk20: u32,
	#[export] pub chain_6k: u32,
	#[export] pub chain_j2s: u32,
	#[export] pub chain_3s: u32,
	#[export] pub chain_3k: u32,
	#[export] pub chain_3h: u32,
	#[export] pub chain_j2h: u32,
	#[export] pub chain_unk27: u32,
	#[export] pub chain_unk28: u32,
	#[export] pub chain_unk29: u32,
	#[export] pub chain_unk30: u32,
	#[export] pub chain_unk31: u32,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// A character's variables and chain tables.
pub struct PlayData {
	base: Base<Resource>,
	#[export] pub variables: Gd<PlayVariables>,
	#[export] pub chain_tables: Array<Gd<ChainTable>>,
}


#[derive(GodotClass)]
#[class(tool, base=Resource)]
/// A scriptable object's script. Contains all of its actions.
pub struct BinScript {
	base: Base<Resource>,
	/// Whether this script has PlayData.
	#[export] pub has_play_data: bool,
	/// Script PlayData.
	#[export] pub play_data: Array<Gd<PlayData>>,
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


#[godot_api] impl IResource for PlayVariables {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			header: 0xE504,
			walk_fwd_x_speed: 0,
			walk_bwd_x_speed: 0,
			dash_x_speed: 0,
			backdash_x_speed: 0,
			backdash_y_speed: 0,
			backdash_gravity: 0,
			jump_fwd_x_speed: 0,
			jump_bwd_x_speed: 0,
			jump_y_speed: 0,
			jump_gravity: 0,
			sjump_fwd_x_speed: 0,
			sjump_bwd_x_speed: 0,
			sjump_y_speed: 0,
			sjump_gravity: 0,
			dash_acceleration: 0,
			dash_resist: 0,
			homing_jump_y_max: 0,
			homing_jump_x_max: 0,
			homing_jump_x_resist: 0,
			homing_target_y_offset: 0,
			airdash_height: 0,
			pd_airdash_fwd_time: 0,
			pd_airdash_bwd_time: 0,
			pd_faint_point: 0,
			pd_defense_point: 0,
			pd_konjo: 0,
			pd_kaishin: 0,
			pd_defense_gravity: 0,
			pd_airdash_count: 0,
			pd_jump_count: 0,
			pd_airdash_fwd_atk_time: 0,
			pd_airdash_bwd_atk_time: 0,
			pd_tension_walk: 0,
			pd_tension_jump: 0,
			pd_tension_dash: 0,
			pd_tension_airdash: 0,
			pdgc_gauge_def_point: 0,
			pdgc_gauge_recovery: 0,
			pd_tension_ib: 0,
			padding: PackedByteArray::new(),
		}
	}
}


#[godot_api] impl IResource for ChainTable {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			chain_5p: 0,
			chain_6p: 0,
			chain_5k: 0,
			chain_fs: 0,
			chain_cs: 0,
			chain_5h: 0,
			chain_6h: 0,
			chain_2p: 0,
			chain_2k: 0,
			chain_2d: 0,
			chain_2s: 0,
			chain_2h: 0,
			chain_jp: 0,
			chain_jk: 0,
			chain_js: 0,
			chain_jh: 0,
			chain_unk16: 0,
			chain_j2k: 0,
			chain_3p: 0,
			chain_unk19: 0,
			chain_unk20: 0,
			chain_6k: 0,
			chain_j2s: 0,
			chain_3s: 0,
			chain_3k: 0,
			chain_3h: 0,
			chain_j2h: 0,
			chain_unk27: 0,
			chain_unk28: 0,
			chain_unk29: 0,
			chain_unk30: 0,
			chain_unk31: 0,
		}
	}
}


#[godot_api] impl IResource for PlayData {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			variables: PlayVariables::new_gd(),
			chain_tables: array![],
		}
	}
}


#[godot_api] impl IResource for BinScript {
	fn init(base: Base<Resource>) -> Self {
		Self {
			base,
			has_play_data: false,
			play_data: array![],
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
			_ => bin_data.extend(u32::to_le_bytes(self.value as u32)),
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


	#[func] pub fn get_animation(&self) -> Gd<Animation> {
		let mut anim: Gd<Animation> = Animation::new_gd();
		anim.set_length(0.00);

		let track_cells		: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_semitrans	: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_scale		: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_scale_y	: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_rotate	: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_draw		: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_cell_jump	: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_visual	: i32 = anim.add_track(animation::TrackType::METHOD);
		let track_end		: i32 = anim.add_track(animation::TrackType::METHOD);

		for track in 0..anim.get_track_count() {
			anim.track_set_path(track, ".");
		}

		// Resets
		anim.track_insert_key(track_semitrans, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_semitrans", 0, 0xFF]
		}.to_variant());

		anim.track_insert_key(track_scale, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_scale", 0, -1]
		}.to_variant());

		anim.track_insert_key(track_scale_y, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_scale", 1, -1]
		}.to_variant());
		
		anim.track_insert_key(track_rotate, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_rotate", 0, 0]
		}.to_variant());
		
		anim.track_insert_key(track_draw, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_draw_normal"]
		}.to_variant());
		
		anim.track_insert_key(track_visual, 0.0, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_visual", 0, 1]
		}.to_variant());
		
		anim.track_insert_key(track_visual, 0.1, &dict!{
			"method": "emit_signal",
			"args": varray!["inst_visual", 3, 0]
		}.to_variant());
		
		let mut frame: i64 = 1;
		let mut frame_offset: i64 = 0;

		for item in self.instructions.iter_shared() {
			let instruction = item.bind();

			// By ID
			match instruction.id {
				ID_CELLBEGIN => {
					frame += frame_offset;

					let cell_length: i64 = max(1, instruction.arguments.at(0).bind().value);
					let cell_number: i64 = instruction.arguments.at(1).bind().value;
					let anim_length: f32 = anim.get_length();

					anim.set_length(anim_length + cell_length as f32);
					anim.track_insert_key(track_cells, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_cell", cell_number]
					}.to_variant());

					frame_offset = cell_length;
				}

				ID_SEMITRANS => {
					let blend_value: i64 = instruction.arguments.at(0).bind().value;
					let blend_mode: i64 = instruction.arguments.at(1).bind().value;

					anim.track_insert_key(track_semitrans, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_semitrans", blend_mode, blend_value]
					}.to_variant());
				}

				ID_SCALE => {
					let scale_mode: i64 = instruction.arguments.at(0).bind().value;
					let scale_value: i64 = instruction.arguments.at(1).bind().value;
					let which_track: i32;

					if scale_mode % 2 == 1 {
						which_track = track_scale_y;
					} else {
						which_track = track_scale;
					}

					anim.track_insert_key(which_track, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_scale", scale_mode, scale_value]
					}.to_variant());
				}

				ID_ROT => {
					let rotate_mode: i64 = instruction.arguments.at(0).bind().value;
					let rotate_value: i64 = instruction.arguments.at(1).bind().value;

					anim.track_insert_key(track_rotate, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_rotate", rotate_mode, rotate_value]
					}.to_variant());
				}

				ID_DRAW_NORMAL => {
					anim.track_insert_key(track_draw, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_draw_normal"]
					}.to_variant());
				}

				ID_DRAW_REVERSE => {
					anim.track_insert_key(track_draw, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_draw_reverse"]
					}.to_variant());
				}

				ID_CELL_JUMP => {
					if instruction.arguments.at(0).bind().value > 0 {
						continue;
					}

					let cell_begin_number: i64 = instruction.arguments.at(2).bind().value;

					anim.track_insert_key(track_cell_jump, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_cell_jump", cell_begin_number]
					}.to_variant());
				}

				ID_VISUAL => {
					let visual_mode: i64 = instruction.arguments.at(0).bind().value;
					let visual_argument: i64 = instruction.arguments.at(1).bind().value;
					let visual_offset: f64;// = 0.0;

					if visual_mode == 1 {
						visual_offset = -0.1;
					} else {
						visual_offset = 0.0;
					}

					anim.track_insert_key(track_visual, frame as f64 + visual_offset, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_visual", visual_mode, visual_argument]
					}.to_variant());
				}

				ID_END_ACTION => {
					let end_mode: i64 = instruction.arguments.at(0).bind().value;

					anim.track_insert_key(track_end, frame as f64, &dict!{
						"method": "emit_signal",
						"args": varray!["inst_end_action", end_mode]
					}.to_variant());
				}

				_ => ()
			}
		}

		return anim;
	}
}


#[godot_api] impl PlayVariables {
	pub fn from_bin(bin_data: &Vec<u8>) -> Gd<Self> {
		let mut cursor: usize = 0x02;
		let header: u16 = u16::from_le_bytes([bin_data[0], bin_data[1]]);

		let mut values: Vec<i16> = Vec::new();
		for _value in 0..39 {
			values.push(
				i16::from_le_bytes([
					bin_data[cursor + 0x00], bin_data[cursor + 0x01]
			]));
			cursor += 0x02;
		}

		// Don't think this doesn't hurt me, too.
		return Gd::from_init_fn(|base| {
			Self {
				base,
				header,
				walk_fwd_x_speed		: values[0x00],
				walk_bwd_x_speed		: values[0x01],
				dash_x_speed			: values[0x02],
				backdash_x_speed		: values[0x03],
				backdash_y_speed		: values[0x04],
				backdash_gravity		: values[0x05],
				jump_fwd_x_speed		: values[0x06],
				jump_bwd_x_speed		: values[0x07],
				jump_y_speed			: values[0x08],
				jump_gravity			: values[0x09],
				sjump_fwd_x_speed		: values[0x0A],
				sjump_bwd_x_speed		: values[0x0B],
				sjump_y_speed			: values[0x0C],
				sjump_gravity			: values[0x0D],
				dash_acceleration		: values[0x0E],
				dash_resist				: values[0x0F],
				homing_jump_y_max		: values[0x10],
				homing_jump_x_max		: values[0x11],
				homing_jump_x_resist	: values[0x12],
				homing_target_y_offset	: values[0x13],
				airdash_height			: values[0x14],
				pd_airdash_fwd_time		: values[0x15],
				pd_airdash_bwd_time		: values[0x16],
				pd_faint_point			: values[0x17],
				pd_defense_point		: values[0x18],
				pd_konjo				: values[0x19],
				pd_kaishin				: values[0x1A],
				pd_defense_gravity		: values[0x1B],
				pd_airdash_count		: values[0x1C],
				pd_jump_count			: values[0x1D],
				pd_airdash_fwd_atk_time	: values[0x1E],
				pd_airdash_bwd_atk_time	: values[0x1F],
				pd_tension_walk			: values[0x20],
				pd_tension_jump			: values[0x21],
				pd_tension_dash			: values[0x22],
				pd_tension_airdash		: values[0x23],
				pdgc_gauge_def_point	: values[0x24],
				pdgc_gauge_recovery		: values[0x25],
				pd_tension_ib			: values[0x26],
				padding					: bin_data[0x50..0x80].to_vec().into()
			}
		});
	}


	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		bin_data.extend(self.header.to_le_bytes());
		bin_data.extend(self.walk_fwd_x_speed.to_le_bytes());
		bin_data.extend(self.walk_bwd_x_speed.to_le_bytes());
		bin_data.extend(self.dash_x_speed.to_le_bytes());
		bin_data.extend(self.backdash_x_speed.to_le_bytes());
		bin_data.extend(self.backdash_y_speed.to_le_bytes());
		bin_data.extend(self.backdash_gravity.to_le_bytes());
		bin_data.extend(self.jump_fwd_x_speed.to_le_bytes());
		bin_data.extend(self.jump_bwd_x_speed.to_le_bytes());
		bin_data.extend(self.jump_y_speed.to_le_bytes());
		bin_data.extend(self.jump_gravity.to_le_bytes());
		bin_data.extend(self.sjump_fwd_x_speed.to_le_bytes());
		bin_data.extend(self.sjump_bwd_x_speed.to_le_bytes());
		bin_data.extend(self.sjump_y_speed.to_le_bytes());
		bin_data.extend(self.sjump_gravity.to_le_bytes());
		bin_data.extend(self.dash_acceleration.to_le_bytes());
		bin_data.extend(self.dash_resist.to_le_bytes());
		bin_data.extend(self.homing_jump_y_max.to_le_bytes());
		bin_data.extend(self.homing_jump_x_max.to_le_bytes());
		bin_data.extend(self.homing_jump_x_resist.to_le_bytes());
		bin_data.extend(self.homing_target_y_offset.to_le_bytes());
		bin_data.extend(self.airdash_height.to_le_bytes());
		bin_data.extend(self.pd_airdash_fwd_time.to_le_bytes());
		bin_data.extend(self.pd_airdash_bwd_time.to_le_bytes());
		bin_data.extend(self.pd_faint_point.to_le_bytes());
		bin_data.extend(self.pd_defense_point.to_le_bytes());
		bin_data.extend(self.pd_konjo.to_le_bytes());
		bin_data.extend(self.pd_kaishin.to_le_bytes());
		bin_data.extend(self.pd_defense_gravity.to_le_bytes());
		bin_data.extend(self.pd_airdash_count.to_le_bytes());
		bin_data.extend(self.pd_jump_count.to_le_bytes());
		bin_data.extend(self.pd_airdash_fwd_atk_time.to_le_bytes());
		bin_data.extend(self.pd_airdash_bwd_atk_time.to_le_bytes());
		bin_data.extend(self.pd_tension_walk.to_le_bytes());
		bin_data.extend(self.pd_tension_jump.to_le_bytes());
		bin_data.extend(self.pd_tension_dash.to_le_bytes());
		bin_data.extend(self.pd_tension_airdash.to_le_bytes());
		bin_data.extend(self.pdgc_gauge_def_point.to_le_bytes());
		bin_data.extend(self.pdgc_gauge_recovery.to_le_bytes());
		bin_data.extend(self.pd_tension_ib.to_le_bytes());
		bin_data.extend(self.padding.to_vec());

		return bin_data;
	}
}


#[godot_api] impl ChainTable {
	pub fn from_bin(bin_data: &Vec<u8>) -> Array<Gd<Self>> {
		let chain_table_count: usize;

		// Looks like if bin_data[0x01] == 0x05
		// there's 5 chain tables, otherwise, there's 1.
		if bin_data[0x01] == 0x05 {
			chain_table_count = 5;
		} else {
			chain_table_count = 1;
		}

		let mut chain_tables: Array<Gd<ChainTable>> = array![];
		let mut cursor = 0x00;

		for _ in 0..chain_table_count {
			cursor += 0x80;
			let chain_bin_data: &Vec<u8> = &bin_data[cursor..(cursor + 0x80)].to_vec();
			chain_tables.push(&Self::from_bin_single(chain_bin_data));
		}

		return chain_tables;
	}

	pub fn from_bin_single(bin_data: &Vec<u8>) -> Gd<Self> {
		let mut cursor: usize = 0x00;
		let mut values: Vec<u32> = Vec::new();

		for _values in 0..32 {
			values.push(u32::from_le_bytes([
				bin_data[cursor + 0x00], bin_data[cursor + 0x01],
				bin_data[cursor + 0x02], bin_data[cursor + 0x03],
			]));
			cursor += 0x04;
		}

		return Gd::from_init_fn(|base| {
			Self {
				base,
				chain_5p	: values[0x00],
				chain_6p	: values[0x01],
				chain_5k	: values[0x02],
				chain_fs	: values[0x03],
				chain_cs	: values[0x04],
				chain_5h	: values[0x05],
				chain_6h	: values[0x06],
				chain_2p	: values[0x07],
				chain_2k	: values[0x08],
				chain_2d	: values[0x09],
				chain_2s	: values[0x0A],
				chain_2h	: values[0x0B],
				chain_jp	: values[0x0C],
				chain_jk	: values[0x0D],
				chain_js	: values[0x0E],
				chain_jh	: values[0x0F],
				chain_unk16	: values[0x10],
				chain_j2k	: values[0x11],
				chain_3p	: values[0x12],
				chain_unk19	: values[0x13],
				chain_unk20	: values[0x14],
				chain_6k	: values[0x15],
				chain_j2s	: values[0x16],
				chain_3s	: values[0x17],
				chain_3k	: values[0x18],
				chain_3h	: values[0x19],
				chain_j2h	: values[0x1A],
				chain_unk27	: values[0x1B],
				chain_unk28	: values[0x1C],
				chain_unk29	: values[0x1D],
				chain_unk30	: values[0x1E],
				chain_unk31	: values[0x1F],
			}
		});
	}


	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		bin_data.extend(self.chain_5p.to_le_bytes());
		bin_data.extend(self.chain_6p.to_le_bytes());
		bin_data.extend(self.chain_5k.to_le_bytes());
		bin_data.extend(self.chain_fs.to_le_bytes());
		bin_data.extend(self.chain_cs.to_le_bytes());
		bin_data.extend(self.chain_5h.to_le_bytes());
		bin_data.extend(self.chain_6h.to_le_bytes());
		bin_data.extend(self.chain_2p.to_le_bytes());
		bin_data.extend(self.chain_2k.to_le_bytes());
		bin_data.extend(self.chain_2d.to_le_bytes());
		bin_data.extend(self.chain_2s.to_le_bytes());
		bin_data.extend(self.chain_2h.to_le_bytes());
		bin_data.extend(self.chain_jp.to_le_bytes());
		bin_data.extend(self.chain_jk.to_le_bytes());
		bin_data.extend(self.chain_js.to_le_bytes());
		bin_data.extend(self.chain_jh.to_le_bytes());
		bin_data.extend(self.chain_unk16.to_le_bytes());
		bin_data.extend(self.chain_j2k.to_le_bytes());
		bin_data.extend(self.chain_3p.to_le_bytes());
		bin_data.extend(self.chain_unk19.to_le_bytes());
		bin_data.extend(self.chain_unk20.to_le_bytes());
		bin_data.extend(self.chain_6k.to_le_bytes());
		bin_data.extend(self.chain_j2s.to_le_bytes());
		bin_data.extend(self.chain_3s.to_le_bytes());
		bin_data.extend(self.chain_3k.to_le_bytes());
		bin_data.extend(self.chain_3h.to_le_bytes());
		bin_data.extend(self.chain_j2h.to_le_bytes());
		bin_data.extend(self.chain_unk27.to_le_bytes());
		bin_data.extend(self.chain_unk28.to_le_bytes());
		bin_data.extend(self.chain_unk29.to_le_bytes());
		bin_data.extend(self.chain_unk30.to_le_bytes());
		bin_data.extend(self.chain_unk31.to_le_bytes());

		return bin_data;
	}
}


#[godot_api] impl PlayData {
	pub fn from_bin(bin_data: &Vec<u8>) -> Gd<Self> {
		return Gd::from_init_fn(|base| {
			Self {
				base,
				variables: PlayVariables::from_bin(bin_data),
				chain_tables: ChainTable::from_bin(bin_data),
			}
		});
	}

	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();
		bin_data.extend(&self.variables.bind().to_bin());

		for table in self.chain_tables.iter_shared() {
			bin_data.extend(table.bind().to_bin());
		}

		return bin_data;
	}
}


#[godot_api] impl BinScript {
	pub fn from_bin(bin_data: Vec<u8>, play_data: bool, instruction_db: &Dictionary) -> Gd<Self> {
		let mut play_data_array: Array<Gd<PlayData>> = array![];
		let mut cursor: usize = 0x00;

		if play_data {
			play_data_array.push(&PlayData::from_bin(&bin_data));

			// Place cursor at end of play data
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
				let bin_data_2: Vec<u8> = bin_data[cursor..cursor * 2].to_vec();
				play_data_array.push(&PlayData::from_bin(&bin_data_2));
				cursor *= 2;
			}
		}

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
				has_play_data: play_data,
				play_data: play_data_array,
				actions,
			}
		})
	}

	pub fn to_bin(&self) -> Vec<u8> {
		let mut bin_data: Vec<u8> = Vec::new();

		for data_set in self.play_data.iter_shared() {
			bin_data.extend(data_set.bind().to_bin());
		}

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

	pub fn from_data(play_data: Array<Gd<PlayData>>, actions: Array<Gd<ScriptAction>>) -> Gd<Self> {
		let has_play_data: bool = play_data.len() > 0;

		return Gd::from_init_fn(|base| {
			Self {
				base,
				has_play_data,
				play_data,
				actions,
			}
		})
	}
}