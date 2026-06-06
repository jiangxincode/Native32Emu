// Action VM: stack-based virtual machine for executing Native32 Action bytecode.
// The VM is stringly-typed (all values are strings on the stack).

use std::collections::HashMap;

use rand::RngExt;

use crate::core::actions::Action;
use crate::core::file_loader::{ActionPayload, Native32Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Default for ActionVM {
    fn default() -> Self {
        Self::new()
    }
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
                        self.rng.random_range(0..upper_val)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::file_loader::Native32Reader;

    // === Helper functions ===

    fn parse_float(s: &str) -> f64 {
        s.parse::<f64>().unwrap_or(0.0)
    }

    /// Mock VmHost that records calls for verification.
    struct MockHost {
        stopped: bool,
        played: bool,
        frame: u32,
        goto_calls: Vec<(String, u32, bool)>,
        properties: std::collections::HashMap<(String, ActionProp), String>,
        cloned: Vec<(String, String, i32)>,
        removed: Vec<String>,
        stop_sounds_called: bool,
        time: u32,
    }

    impl MockHost {
        fn new() -> Self {
            Self {
                stopped: false,
                played: false,
                frame: 1,
                goto_calls: Vec::new(),
                properties: std::collections::HashMap::new(),
                cloned: Vec::new(),
                removed: Vec::new(),
                stop_sounds_called: false,
                time: 0,
            }
        }
    }

    impl VmHost for MockHost {
        fn stop(&mut self, _target: &str) {
            self.stopped = true;
        }
        fn play(&mut self, _target: &str) {
            self.played = true;
        }
        fn get_frame(&mut self, _target: &str) -> u32 {
            self.frame
        }
        fn goto_frame(&mut self, target: &str, frame: u32, playing: bool) {
            self.goto_calls.push((target.to_string(), frame, playing));
        }
        fn stop_sounds(&mut self, _target: &str) {
            self.stop_sounds_called = true;
        }
        fn set_property(&mut self, target: &str, prop: ActionProp, value: &str) {
            self.properties
                .insert((target.to_string(), prop), value.to_string());
        }
        fn get_property(&mut self, target: &str, prop: ActionProp) -> String {
            self.properties
                .get(&(target.to_string(), prop))
                .cloned()
                .unwrap_or_else(|| "0".to_string())
        }
        fn clone_sprite(&mut self, src: &str, dest: &str, depth: i32) {
            self.cloned.push((src.to_string(), dest.to_string(), depth));
        }
        fn remove_sprite(&mut self, name: &str) {
            self.removed.push(name.to_string());
        }
        fn call(&mut self, _frame: u32) {}
        fn get_time(&self) -> u32 {
            self.time
        }
        fn get_url(&mut self, _url: &str, _target: &str) {}
        fn run_frame_actions(&mut self, _frame: u32) {}
    }

    /// Build a Native32Reader with pre-loaded action bytecode for testing.
    ///
    /// Layout:
    ///   base = 0x60
    ///   actions start at buffer offset 0x60 (action_idx = 0 relative to base)
    ///   each action = 8 bytes: opcode(u32 LE) + payload(u32 LE)
    ///   payload string data placed after all actions
    fn make_test_reader(actions: &[(Action, Option<ActionPayload>)]) -> Native32Reader {
        let base = 0x60usize;
        let action_count = actions.len();
        // Each action: 8 bytes instruction + up to 64 bytes string payload
        let data_size = base + action_count * 8 + action_count * 64 + 128;
        let mut data = vec![0u8; data_size];

        // Place actions
        for (i, (op, payload)) in actions.iter().enumerate() {
            let offset = base + i * 8;
            data[offset..offset + 4].copy_from_slice(&(*op as u32).to_le_bytes());
            match payload {
                None => {}
                Some(ActionPayload::Integer(val)) => {
                    // For integer payloads, write the i16 value directly at a known location
                    // Place it right after the action table
                    let payload_offset = base + action_count * 8 + i * 4;
                    data[payload_offset..payload_offset + 2].copy_from_slice(&val.to_le_bytes());
                    let rel_offset = (payload_offset - base) as u32;
                    data[offset + 4..offset + 8].copy_from_slice(&rel_offset.to_le_bytes());
                }
                Some(ActionPayload::String(s)) => {
                    // Place string after the action table
                    let payload_offset = base + action_count * 8 + i * 64;
                    let bytes = s.as_bytes();
                    data[payload_offset..payload_offset + bytes.len()].copy_from_slice(bytes);
                    data[payload_offset + bytes.len()] = 0; // null terminator
                    let rel_offset = (payload_offset - base) as u32;
                    data[offset + 4..offset + 8].copy_from_slice(&rel_offset.to_le_bytes());
                }
            }
        }

        let mut reader = Native32Reader::new(data);
        reader.base = base;
        reader
    }

    /// Run VM with given actions and return the mock host for inspection.
    fn run_vm(actions: &[(Action, Option<ActionPayload>)]) -> MockHost {
        let mut reader = make_test_reader(actions);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        vm.run(&mut reader, &mut host, 1, "");
        host
    }

    fn run_vm_vars(actions: &[(Action, Option<ActionPayload>)]) -> (ActionVM, MockHost) {
        let mut reader = make_test_reader(actions);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        vm.run(&mut reader, &mut host, 1, "");
        (vm, host)
    }

    /// Helper: compute with 2 operands, store result in variable `r`.
    /// The pattern is: Push "r", Push a, Push b, Op, SetVariable, End
    fn compute_2op(a: &str, b: &str, op: Action) -> ActionVM {
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String(a.to_string()))),
            (Action::Push, Some(ActionPayload::String(b.to_string()))),
            (op, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        vm
    }

    /// Helper: compute with 1 operand, store result in variable `r`.
    fn compute_1op(a: &str, op: Action) -> ActionVM {
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String(a.to_string()))),
            (op, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        vm
    }

    /// Helper: set var to val_a, then compute val_a op val_b, store in `r`.
    fn compute_2op_with_var(var: &str, val_a: &str, val_b: &str, op: Action) -> ActionVM {
        let (vm, _) = run_vm_vars(&[
            // Set var = val_a
            (Action::Push, Some(ActionPayload::String(var.to_string()))),
            (Action::Push, Some(ActionPayload::String(val_a.to_string()))),
            (Action::SetVariable, None),
            // Push "r", get var, push val_b, op, set r
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String(var.to_string()))),
            (Action::GetVariable, None),
            (Action::Push, Some(ActionPayload::String(val_b.to_string()))),
            (op, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        vm
    }

    // === Pure helper function tests ===

    #[test]
    fn test_str_to_float_valid() {
        assert_eq!(str_to_float("3.25"), 3.25);
        assert_eq!(str_to_float("0"), 0.0);
        assert_eq!(str_to_float("-1.5"), -1.5);
    }

    #[test]
    fn test_str_to_float_empty() {
        assert_eq!(str_to_float(""), 0.0);
    }

    #[test]
    fn test_str_to_float_invalid() {
        assert_eq!(str_to_float("abc"), 0.0);
    }

    #[test]
    fn test_str_to_int_valid() {
        assert_eq!(str_to_int("42"), 42);
        assert_eq!(str_to_int("3.9"), 3); // truncated
        assert_eq!(str_to_int("-10"), -10);
    }

    #[test]
    fn test_str_to_int_empty() {
        assert_eq!(str_to_int(""), 0);
    }

    #[test]
    fn test_to_string_fn() {
        assert_eq!(to_string(42), "42");
        assert_eq!(to_string(3.25f64), "3.25");
        assert_eq!(to_string("hello"), "hello");
    }

    #[test]
    fn test_action_prop_from_u32() {
        assert_eq!(ActionProp::from_u32(0), Some(ActionProp::X));
        assert_eq!(ActionProp::from_u32(1), Some(ActionProp::Y));
        assert_eq!(ActionProp::from_u32(4), Some(ActionProp::CurrentFrame));
        assert_eq!(ActionProp::from_u32(7), Some(ActionProp::Visible));
        assert_eq!(ActionProp::from_u32(13), Some(ActionProp::Name));
        assert_eq!(ActionProp::from_u32(10), None); // unused
        assert_eq!(ActionProp::from_u32(99), None);
    }

    // === VM opcode tests ===
    //
    // SetVariable semantics: pops val (top), pops var_name (second), inserts vars[var] = val.
    // Correct pattern: Push var_name, ..., Push value, SetVariable
    // For computed values: Push var_name, Push operands, Op, SetVariable

    // --- Stack operations ---

    #[test]
    fn test_vm_end_immediately() {
        let (vm, _) = run_vm_vars(&[(Action::End, None)]);
        assert!(vm.vars.is_empty());
    }

    #[test]
    fn test_vm_push_string_and_set() {
        // Push var name, push value, SetVariable
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("x".to_string()))),
            (
                Action::Push,
                Some(ActionPayload::String("hello".to_string())),
            ),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("x"), Some(&"hello".to_string()));
    }

    #[test]
    fn test_vm_push_integer_and_set() {
        // Push always reads string payload from the reader (even for Integer variant,
        // the reader's disassemble_action treats Push as string). Use String("42").
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("x".to_string()))),
            (Action::Push, Some(ActionPayload::String("42".to_string()))),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("x"), Some(&"42".to_string()));
    }

    #[test]
    fn test_vm_pop() {
        // Push a, Push b, Pop (removes b), Push var, Push a (still on stack? no, pop removed b)
        // Actually: Push "a", Push "b", Pop → stack: ["a"]
        // Then Push "v", Push "a" is gone... let me rethink.
        // Simple: Push "keep", Push "discard", Pop, Push "v", SetVariable
        // Stack after Push "keep": ["keep"]
        // Stack after Push "discard": ["keep", "discard"]
        // Stack after Pop: ["keep"]
        // Stack after Push "v": ["keep", "v"]
        // SetVariable: val = "v", var = "keep" → vars["keep"] = "v"
        let (vm, _) = run_vm_vars(&[
            (
                Action::Push,
                Some(ActionPayload::String("keep".to_string())),
            ),
            (
                Action::Push,
                Some(ActionPayload::String("discard".to_string())),
            ),
            (Action::Pop, None),
            (Action::Push, Some(ActionPayload::String("v".to_string()))),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("keep"), Some(&"v".to_string()));
    }

    // --- Arithmetic ---

    #[test]
    fn test_vm_add() {
        let vm = compute_2op("3", "5", Action::Add);
        assert_eq!(vm.vars.get("r"), Some(&"8".to_string()));
    }

    #[test]
    fn test_vm_subtract() {
        let vm = compute_2op("10", "3", Action::Subtract);
        assert_eq!(vm.vars.get("r"), Some(&"7".to_string()));
    }

    #[test]
    fn test_vm_multiply() {
        let vm = compute_2op("4", "5", Action::Multiply);
        assert_eq!(vm.vars.get("r"), Some(&"20".to_string()));
    }

    #[test]
    fn test_vm_divide() {
        let vm = compute_2op("20", "4", Action::Divide);
        assert_eq!(vm.vars.get("r"), Some(&"5".to_string()));
    }

    #[test]
    fn test_vm_divide_by_zero() {
        let vm = compute_2op("10", "0", Action::Divide);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    // --- Comparison ---

    #[test]
    fn test_vm_equals_true() {
        let vm = compute_2op("5", "5", Action::Equals);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_equals_false() {
        let vm = compute_2op("5", "6", Action::Equals);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    #[test]
    fn test_vm_less_true() {
        let vm = compute_2op("3", "5", Action::Less);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_less_false() {
        let vm = compute_2op("5", "3", Action::Less);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    // --- Logic ---

    #[test]
    fn test_vm_and_both_true() {
        let vm = compute_2op("1", "1", Action::And);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_and_one_false() {
        let vm = compute_2op("1", "0", Action::And);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    #[test]
    fn test_vm_or() {
        let vm = compute_2op("0", "1", Action::Or);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_not_true() {
        let vm = compute_1op("0", Action::Not);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_not_false() {
        let vm = compute_1op("42", Action::Not);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    // --- String operations ---

    #[test]
    fn test_vm_string_equals() {
        let vm = compute_2op("abc", "abc", Action::StringEquals);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    #[test]
    fn test_vm_string_equals_different() {
        let vm = compute_2op("abc", "def", Action::StringEquals);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    #[test]
    fn test_vm_string_add() {
        let vm = compute_2op("hello", " world", Action::StringAdd);
        assert_eq!(vm.vars.get("r"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_vm_string_length() {
        let vm = compute_1op("hello", Action::StringLength);
        assert_eq!(vm.vars.get("r"), Some(&"5".to_string()));
    }

    #[test]
    fn test_vm_string_extract() {
        // StringExtract: pop len, pop start, pop string → push substring
        // "hello", start=2, len=3 → "ell" (1-indexed)
        // Stack order for StringExtract: [string, start, len] with len on top
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (
                Action::Push,
                Some(ActionPayload::String("hello".to_string())),
            ),
            (Action::Push, Some(ActionPayload::String("2".to_string()))),
            (Action::Push, Some(ActionPayload::String("3".to_string()))),
            (Action::StringExtract, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"ell".to_string()));
    }

    #[test]
    fn test_vm_string_less() {
        let vm = compute_2op("abc", "def", Action::StringLess);
        assert_eq!(vm.vars.get("r"), Some(&"1".to_string()));
    }

    // --- Type conversion ---

    #[test]
    fn test_vm_to_integer() {
        let vm = compute_1op("3.14", Action::ToInteger);
        assert_eq!(vm.vars.get("r"), Some(&"3".to_string()));
    }

    #[test]
    fn test_vm_char_to_ascii() {
        let vm = compute_1op("A", Action::CharToAscii);
        assert_eq!(vm.vars.get("r"), Some(&"65".to_string()));
    }

    #[test]
    fn test_vm_ascii_to_char() {
        let vm = compute_1op("65", Action::AsciiToChar);
        assert_eq!(vm.vars.get("r"), Some(&"A".to_string()));
    }

    // --- Variable operations ---

    #[test]
    fn test_vm_set_and_get_variable() {
        // Set score=42, then verify it's stored
        let (vm, _) = run_vm_vars(&[
            (
                Action::Push,
                Some(ActionPayload::String("score".to_string())),
            ),
            (Action::Push, Some(ActionPayload::String("42".to_string()))),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("score"), Some(&"42".to_string()));
    }

    #[test]
    fn test_vm_get_variable_pushes_onto_stack() {
        // Set score=42, get score, use in Add with 8 → 50
        let vm = compute_2op_with_var("score", "42", "8", Action::Add);
        assert_eq!(vm.vars.get("r"), Some(&"50".to_string()));
    }

    #[test]
    fn test_vm_get_undefined_variable_pushes_empty() {
        // GetVariable for undefined var → pushes "" → StringLength → 0
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (
                Action::Push,
                Some(ActionPayload::String("nonexistent".to_string())),
            ),
            (Action::GetVariable, None),  // pushes ""
            (Action::StringLength, None), // "" → length 0
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    // --- Control flow ---

    #[test]
    fn test_vm_if_true_branches() {
        // Push 1 (true), If with offset=1 → skip next instruction
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String("1".to_string()))),
            (Action::If, Some(ActionPayload::Integer(1))), // true, skip 1
            (
                Action::Push,
                Some(ActionPayload::String("skipped".to_string())),
            ), // skipped
            (
                Action::Push,
                Some(ActionPayload::String("reached".to_string())),
            ), // reached
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"reached".to_string()));
    }

    #[test]
    fn test_vm_if_false_falls_through() {
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String("0".to_string()))),
            (Action::If, Some(ActionPayload::Integer(1))), // false, don't branch
            (
                Action::Push,
                Some(ActionPayload::String("reached".to_string())),
            ),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"reached".to_string()));
    }

    #[test]
    fn test_vm_jump() {
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Jump, Some(ActionPayload::Integer(1))),
            (
                Action::Push,
                Some(ActionPayload::String("skipped".to_string())),
            ),
            (
                Action::Push,
                Some(ActionPayload::String("reached".to_string())),
            ),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"reached".to_string()));
    }

    // --- Host interaction ---

    #[test]
    fn test_vm_stop() {
        let host = run_vm(&[(Action::Stop, None), (Action::End, None)]);
        assert!(host.stopped);
    }

    #[test]
    fn test_vm_play() {
        let host = run_vm(&[(Action::Play, None), (Action::End, None)]);
        assert!(host.played);
    }

    #[test]
    fn test_vm_stop_sounds() {
        let host = run_vm(&[(Action::StopSounds, None), (Action::End, None)]);
        assert!(host.stop_sounds_called);
    }

    #[test]
    fn test_vm_next_frame() {
        let host = run_vm(&[(Action::NextFrame, None), (Action::End, None)]);
        assert_eq!(host.goto_calls.len(), 1);
        assert_eq!(host.goto_calls[0].1, 2); // current_frame(1) + 1
    }

    #[test]
    fn test_vm_previous_frame() {
        let host = run_vm(&[(Action::PreviousFrame, None), (Action::End, None)]);
        assert_eq!(host.goto_calls.len(), 1);
        assert_eq!(host.goto_calls[0].1, 0); // 1 - 1
    }

    #[test]
    fn test_vm_previous_frame_at_zero() {
        let mut reader = make_test_reader(&[(Action::PreviousFrame, None), (Action::End, None)]);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        host.frame = 0;
        vm.run(&mut reader, &mut host, 1, "");
        assert_eq!(host.goto_calls[0].1, 0); // saturating_sub
    }

    #[test]
    fn test_vm_goto_frame() {
        let host = run_vm(&[
            (Action::GotoFrame, Some(ActionPayload::Integer(9))),
            (Action::End, None),
        ]);
        assert_eq!(host.goto_calls.len(), 1);
        assert_eq!(host.goto_calls[0].1, 10); // frame + 1 (1-based)
    }

    #[test]
    fn test_vm_goto_frame2() {
        let host = run_vm(&[
            (Action::Push, Some(ActionPayload::String("5".to_string()))),
            (Action::GotoFrame2, None),
            (Action::End, None),
        ]);
        assert_eq!(host.goto_calls.len(), 1);
        assert_eq!(host.goto_calls[0].1, 5);
    }

    #[test]
    fn test_vm_clone_sprite() {
        let host = run_vm(&[
            (Action::Push, Some(ActionPayload::String("src".to_string()))),
            (
                Action::Push,
                Some(ActionPayload::String("dest".to_string())),
            ),
            (Action::Push, Some(ActionPayload::String("10".to_string()))),
            (Action::CloneSprite, None),
            (Action::End, None),
        ]);
        assert_eq!(host.cloned.len(), 1);
        assert_eq!(host.cloned[0], ("src".to_string(), "dest".to_string(), 10));
    }

    #[test]
    fn test_vm_remove_sprite() {
        let host = run_vm(&[
            (
                Action::Push,
                Some(ActionPayload::String("target".to_string())),
            ),
            (Action::RemoveSprite, None),
            (Action::End, None),
        ]);
        assert_eq!(host.removed, vec!["target"]);
    }

    // --- Edge cases ---

    #[test]
    fn test_vm_empty_actions() {
        let (vm, _) = run_vm_vars(&[]);
        assert!(vm.vars.is_empty());
    }

    #[test]
    fn test_vm_pop_empty_stack() {
        let (vm, _) = run_vm_vars(&[(Action::Pop, None), (Action::End, None)]);
        assert!(vm.vars.is_empty());
    }

    #[test]
    fn test_vm_set_variable_empty_stack() {
        let (vm, _) = run_vm_vars(&[(Action::SetVariable, None), (Action::End, None)]);
        assert_eq!(vm.vars.get(""), Some(&"".to_string()));
    }

    #[test]
    fn test_vm_set_target() {
        let mut reader = make_test_reader(&[
            (
                Action::SetTarget,
                Some(ActionPayload::String("movie1".to_string())),
            ),
            (Action::Stop, None),
            (Action::End, None),
        ]);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        vm.run(&mut reader, &mut host, 1, "");
        assert!(host.stopped);
    }

    #[test]
    fn test_vm_set_target2() {
        let mut reader = make_test_reader(&[
            (
                Action::Push,
                Some(ActionPayload::String("movie2".to_string())),
            ),
            (Action::SetTarget2, None),
            (Action::Play, None),
            (Action::End, None),
        ]);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        vm.run(&mut reader, &mut host, 1, "");
        assert!(host.played);
    }

    #[test]
    fn test_vm_get_time() {
        let mut reader = make_test_reader(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::GetTime, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        let mut vm = ActionVM::new();
        let mut host = MockHost::new();
        host.time = 12345;
        vm.run(&mut reader, &mut host, 1, "");
        assert_eq!(vm.vars.get("r"), Some(&"12345".to_string()));
    }

    #[test]
    fn test_vm_random_number() {
        // RandomNumber(10) should return 0..9
        let vm = compute_1op("10", Action::RandomNumber);
        let val: i64 = vm.vars.get("r").unwrap().parse().unwrap();
        assert!((0..10).contains(&val));
    }

    #[test]
    fn test_vm_random_number_zero() {
        let vm = compute_1op("0", Action::RandomNumber);
        assert_eq!(vm.vars.get("r"), Some(&"0".to_string()));
    }

    // --- Complex sequences ---

    #[test]
    fn test_vm_arithmetic_chain() {
        // (3 + 5) * 2 = 16
        // Push "r", Push 3, Push 5, Add → ["r", 8], Push 2, Multiply → ["r", 16], SetVariable
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("r".to_string()))),
            (Action::Push, Some(ActionPayload::String("3".to_string()))),
            (Action::Push, Some(ActionPayload::String("5".to_string()))),
            (Action::Add, None),
            (Action::Push, Some(ActionPayload::String("2".to_string()))),
            (Action::Multiply, None),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("r"), Some(&"16".to_string()));
    }

    #[test]
    fn test_vm_conditional_logic() {
        // if (3 < 5) then x=1 else x=0
        let (vm, _) = run_vm_vars(&[
            (Action::Push, Some(ActionPayload::String("x".to_string()))),
            (Action::Push, Some(ActionPayload::String("3".to_string()))),
            (Action::Push, Some(ActionPayload::String("5".to_string()))),
            (Action::Less, None),                          // 3 < 5 → 1
            (Action::If, Some(ActionPayload::Integer(1))), // true, skip 1
            (Action::Push, Some(ActionPayload::String("0".to_string()))), // skipped
            (Action::Push, Some(ActionPayload::String("1".to_string()))),
            (Action::SetVariable, None),
            (Action::End, None),
        ]);
        assert_eq!(vm.vars.get("x"), Some(&"1".to_string()));
    }
}
