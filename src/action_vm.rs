// Action VM: stack-based virtual machine for executing Native32 Action bytecode.
// The VM is stringly-typed (all values are strings on the stack).

use std::collections::HashMap;

use rand::Rng;

use crate::actions::Action;
use crate::file_loader::{ActionPayload, Native32Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ActionProp {
    X = 0,
    Y = 1,
    XScale = 2,
    YScale = 3,
    CurrentFrame = 4,
    TotalFrames = 5,
    Alpha = 6,
    Visible = 7,
    Width = 8,
    Height = 9,
    Name = 13,
}

impl ActionProp {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(ActionProp::X),
            1 => Some(ActionProp::Y),
            2 => Some(ActionProp::XScale),
            3 => Some(ActionProp::YScale),
            4 => Some(ActionProp::CurrentFrame),
            5 => Some(ActionProp::TotalFrames),
            6 => Some(ActionProp::Alpha),
            7 => Some(ActionProp::Visible),
            8 => Some(ActionProp::Width),
            9 => Some(ActionProp::Height),
            13 => Some(ActionProp::Name),
            _ => None,
        }
    }
}

fn str_to_float(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    s.parse::<f64>().unwrap_or(0.0)
}

fn str_to_int(s: &str) -> i64 {
    str_to_float(s) as i64
}

fn to_string<T: ToString>(v: T) -> String {
    v.to_string()
}

/// Trait for the emulator backend that the VM operates on.
/// This decouples the VM from the full emulator state.
pub trait VmHost {
    fn stop(&mut self, target: &str);
    fn play(&mut self, target: &str);
    fn get_frame(&mut self, target: &str) -> u32;
    fn goto_frame(&mut self, target: &str, frame: u32, playing: bool);
    fn stop_sounds(&mut self, target: &str);
    fn set_property(&mut self, target: &str, prop: ActionProp, value: &str);
    fn get_property(&mut self, target: &str, prop: ActionProp) -> String;
    fn clone_sprite(&mut self, src: &str, dest: &str, depth: i32);
    fn remove_sprite(&mut self, name: &str);
    fn call(&mut self, frame: u32);
    fn get_time(&self) -> u32;
    fn get_url(&mut self, url: &str, target: &str);
    fn run_frame_actions(&mut self, frame: u32);
}

pub struct ActionVM {
    pub vars: HashMap<String, String>,
    rng: rand::rngs::StdRng,
}

impl ActionVM {
    pub fn new() -> Self {
        use rand::SeedableRng;
        Self {
            vars: HashMap::new(),
            rng: rand::rngs::StdRng::seed_from_u64(0),
        }
    }

    /// Execute actions starting at the given 1-based instruction index.
    pub fn run<H: VmHost>(
        &mut self,
        reader: &mut Native32Reader,
        host: &mut H,
        index: u32,
        target: &str,
    ) {
        let mut pc = index;
        let mut stack: Vec<String> = Vec::new();
        let mut current_target = target.to_string();

        loop {
            let npc = pc + 1;
            let action = reader.get_action(pc);
            if action.is_none() {
                break;
            }
            let (op, ref payload) = *action.as_ref().unwrap();

            match op {
                Action::End => break,

                Action::Push => {
                    if let Some(ref p) = payload {
                        match p {
                            ActionPayload::String(s) => stack.push(s.clone()),
                            ActionPayload::Integer(n) => stack.push(n.to_string()),
                        }
                    }
                }

                Action::Pop => {
                    stack.pop();
                }

                Action::SetVariable => {
                    let val = stack.pop().unwrap_or_default();
                    let var = stack.pop().unwrap_or_default();
                    log::trace!("  {} = {}", var, val);
                    self.vars.insert(var.to_lowercase(), val);
                }

                Action::GetVariable => {
                    let name = stack.pop().unwrap_or_default();
                    let val = self
                        .vars
                        .get(&name.to_lowercase())
                        .cloned()
                        .unwrap_or_default();
                    stack.push(val);
                }

                Action::Add => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = str_to_float(&a) + str_to_float(&b);
                    stack.push(to_string(result));
                }

                Action::Subtract => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = str_to_float(&a) - str_to_float(&b);
                    stack.push(to_string(result));
                }

                Action::Multiply => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = str_to_float(&a) * str_to_float(&b);
                    stack.push(to_string(result));
                }

                Action::Divide => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let b_val = str_to_float(&b);
                    let result = if b_val != 0.0 {
                        str_to_float(&a) / b_val
                    } else {
                        0.0
                    };
                    stack.push(to_string(result));
                }

                Action::Equals => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if str_to_float(&a) == str_to_float(&b) {
                        1
                    } else {
                        0
                    };
                    stack.push(to_string(result));
                }

                Action::Less => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if str_to_float(&a) < str_to_float(&b) {
                        1
                    } else {
                        0
                    };
                    stack.push(to_string(result));
                }

                Action::And => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if str_to_int(&a) != 0 && str_to_int(&b) != 0 {
                        1
                    } else {
                        0
                    };
                    stack.push(to_string(result));
                }

                Action::Or => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if str_to_int(&a) != 0 || str_to_int(&b) != 0 {
                        1
                    } else {
                        0
                    };
                    stack.push(to_string(result));
                }

                Action::Not => {
                    let a = stack.pop().unwrap_or_default();
                    let result = if str_to_int(&a) == 0 { 1 } else { 0 };
                    stack.push(to_string(result));
                }

                Action::StringEquals => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if a == b { 1 } else { 0 };
                    stack.push(to_string(result));
                }

                Action::StringAdd => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    stack.push(format!("{}{}", a, b));
                }

                Action::StringLess => {
                    let b = stack.pop().unwrap_or_default();
                    let a = stack.pop().unwrap_or_default();
                    let result = if a < b { 1 } else { 0 };
                    stack.push(to_string(result));
                }

                Action::StringLength => {
                    let a = stack.pop().unwrap_or_default();
                    stack.push(to_string(a.len()));
                }

                Action::StringExtract => {
                    let len = stack.pop().unwrap_or_default();
                    let start = stack.pop().unwrap_or_default();
                    let s = stack.pop().unwrap_or_default();
                    let start_idx = (str_to_int(&start) - 1).max(0) as usize;
                    let end_idx = start_idx + str_to_int(&len) as usize;
                    let result = if start_idx < s.len() {
                        &s[start_idx..end_idx.min(s.len())]
                    } else {
                        ""
                    };
                    stack.push(result.to_string());
                }

                Action::ToInteger => {
                    let a = stack.pop().unwrap_or_default();
                    stack.push(to_string(str_to_int(&a)));
                }

                Action::CharToAscii => {
                    let a = stack.pop().unwrap_or_default();
                    let result = a.chars().next().map(|c| c as u32).unwrap_or(0);
                    stack.push(to_string(result));
                }

                Action::AsciiToChar => {
                    let a = stack.pop().unwrap_or_default();
                    let code = str_to_int(&a) as u32;
                    let c = char::from_u32(code).unwrap_or('\0');
                    stack.push(c.to_string());
                }

                Action::Jump => {
                    if let Some(ActionPayload::Integer(offset)) = payload {
                        let new_pc = if *offset >= 0 {
                            (pc as i64 + *offset as i64 + 1) as u32
                        } else {
                            (pc as i64 + *offset as i64) as u32
                        };
                        pc = new_pc;
                        continue;
                    }
                }

                Action::If => {
                    let cond = stack.pop().unwrap_or_default();
                    let cond_val = str_to_float(&cond) as i64;
                    if cond_val != 0 {
                        if let Some(ActionPayload::Integer(offset)) = payload {
                            let new_pc = if *offset >= 0 {
                                (pc as i64 + *offset as i64 + 1) as u32
                            } else {
                                (pc as i64 + *offset as i64) as u32
                            };
                            pc = new_pc;
                            continue;
                        }
                    }
                }

                Action::Stop => {
                    host.stop(&current_target);
                }

                Action::Play => {
                    host.play(&current_target);
                }

                Action::StopSounds => {
                    host.stop_sounds(&current_target);
                }

                Action::NextFrame => {
                    let f = host.get_frame(&current_target);
                    host.goto_frame(&current_target, f + 1, false);
                }

                Action::PreviousFrame => {
                    let f = host.get_frame(&current_target);
                    host.goto_frame(&current_target, f.saturating_sub(1), false);
                }

                Action::GotoFrame => {
                    if let Some(ActionPayload::Integer(frame)) = payload {
                        host.goto_frame(&current_target, *frame as u32 + 1, false);
                    }
                }

                Action::GotoFrame2 => {
                    let frame = stack.pop().unwrap_or_default();
                    let frame_num = str_to_float(&frame) as u32;
                    let playing = if let Some(ActionPayload::Integer(flags)) = payload {
                        (flags & 0x1) != 0
                    } else {
                        false
                    };
                    host.goto_frame(&current_target, frame_num, playing);
                }

                Action::SetTarget => {
                    if let Some(ActionPayload::String(s)) = payload {
                        current_target = s.clone();
                    }
                }

                Action::SetTarget2 => {
                    let t = stack.pop().unwrap_or_default();
                    current_target = t;
                }

                Action::SetProperty => {
                    let value = stack.pop().unwrap_or_default();
                    let prop_val = stack.pop().unwrap_or_default();
                    let target_name = stack.pop().unwrap_or_default();
                    if let Some(prop) = ActionProp::from_u32(str_to_int(&prop_val) as u32) {
                        log::trace!("SetProperty({}, {:?}, {})", target_name, prop, value);
                        host.set_property(&target_name, prop, &value);
                    }
                }

                Action::GetProperty => {
                    let prop_val = stack.pop().unwrap_or_default();
                    let target_name = stack.pop().unwrap_or_default();
                    if let Some(prop) = ActionProp::from_u32(str_to_int(&prop_val) as u32) {
                        let result = host.get_property(&target_name, prop);
                        log::trace!("GetProperty({}, {:?}) -> {}", target_name, prop, result);
                        stack.push(result);
                    } else {
                        stack.push("0".to_string());
                    }
                }

                Action::CloneSprite => {
                    let depth = stack.pop().unwrap_or_default();
                    let dest = stack.pop().unwrap_or_default();
                    let src = stack.pop().unwrap_or_default();
                    host.clone_sprite(&src, &dest, str_to_int(&depth) as i32);
                }

                Action::RemoveSprite => {
                    let name = stack.pop().unwrap_or_default();
                    host.remove_sprite(&name);
                }

                Action::Call => {
                    let frame = stack.pop().unwrap_or_default();
                    host.call(str_to_int(&frame) as u32);
                }

                Action::RandomNumber => {
                    let upper = stack.pop().unwrap_or_default();
                    let upper_val = str_to_int(&upper) as u32;
                    let result = if upper_val > 0 {
                        self.rng.gen_range(0..upper_val)
                    } else {
                        0
                    };
                    stack.push(to_string(result));
                }

                Action::GetTime => {
                    stack.push(to_string(host.get_time()));
                }

                Action::GetUrl2 => {
                    let target_val = stack.pop().unwrap_or_default();
                    let url = stack.pop().unwrap_or_default();
                    host.get_url(&url, &target_val);
                }

                Action::Trace => {
                    let msg = stack.pop().unwrap_or_default();
                    log::trace!("Trace: {}", msg);
                }

                _ => {
                    log::warn!("Unhandled action opcode {:?} at pc={}", op, pc);
                }
            }

            pc = npc;
        }
    }
}
