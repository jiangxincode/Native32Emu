// Action bytecode opcodes for the Native32 virtual machine.
// These correspond to ActionScript opcodes with a simplified bytecode encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Action {
    End = 0x00,
    NextFrame = 0x04,
    PreviousFrame = 0x05,
    Play = 0x06,
    Stop = 0x07,
    StopSounds = 0x09,
    Add = 0x0a,
    Subtract = 0x0b,
    Multiply = 0x0c,
    Divide = 0x0d,
    Equals = 0x0e,
    Less = 0x0f,
    And = 0x10,
    Or = 0x11,
    Not = 0x12,
    StringEquals = 0x13,
    StringLength = 0x14,
    StringExtract = 0x15,
    Pop = 0x17,
    ToInteger = 0x18,
    GetVariable = 0x1c,
    SetVariable = 0x1d,
    SetTarget2 = 0x20,
    StringAdd = 0x21,
    GetProperty = 0x22,
    SetProperty = 0x23,
    CloneSprite = 0x24,
    RemoveSprite = 0x25,
    Trace = 0x26,
    StartDrag = 0x27,
    EndDrag = 0x28,
    StringLess = 0x29,
    RandomNumber = 0x30,
    MBStringLength = 0x31,
    CharToAscii = 0x32,
    AsciiToChar = 0x33,
    GetTime = 0x34,
    MBStringExtract = 0x35,
    MBCharToAscii = 0x36,
    MBAsciiToChar = 0x37,
    GotoFrame = 0x81,
    WaitForFrame = 0x8a,
    SetTarget = 0x8b,
    GotoLabel = 0x8c,
    WaitForFrame2 = 0x8d,
    Push = 0x96,
    Jump = 0x99,
    GetUrl2 = 0x9a,
    If = 0x9d,
    Call = 0x9e,
    GotoFrame2 = 0x9f,
}

impl Action {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0x00 => Some(Action::End),
            0x04 => Some(Action::NextFrame),
            0x05 => Some(Action::PreviousFrame),
            0x06 => Some(Action::Play),
            0x07 => Some(Action::Stop),
            0x09 => Some(Action::StopSounds),
            0x0a => Some(Action::Add),
            0x0b => Some(Action::Subtract),
            0x0c => Some(Action::Multiply),
            0x0d => Some(Action::Divide),
            0x0e => Some(Action::Equals),
            0x0f => Some(Action::Less),
            0x10 => Some(Action::And),
            0x11 => Some(Action::Or),
            0x12 => Some(Action::Not),
            0x13 => Some(Action::StringEquals),
            0x14 => Some(Action::StringLength),
            0x15 => Some(Action::StringExtract),
            0x17 => Some(Action::Pop),
            0x18 => Some(Action::ToInteger),
            0x1c => Some(Action::GetVariable),
            0x1d => Some(Action::SetVariable),
            0x20 => Some(Action::SetTarget2),
            0x21 => Some(Action::StringAdd),
            0x22 => Some(Action::GetProperty),
            0x23 => Some(Action::SetProperty),
            0x24 => Some(Action::CloneSprite),
            0x25 => Some(Action::RemoveSprite),
            0x26 => Some(Action::Trace),
            0x27 => Some(Action::StartDrag),
            0x28 => Some(Action::EndDrag),
            0x29 => Some(Action::StringLess),
            0x30 => Some(Action::RandomNumber),
            0x31 => Some(Action::MBStringLength),
            0x32 => Some(Action::CharToAscii),
            0x33 => Some(Action::AsciiToChar),
            0x34 => Some(Action::GetTime),
            0x35 => Some(Action::MBStringExtract),
            0x36 => Some(Action::MBCharToAscii),
            0x37 => Some(Action::MBAsciiToChar),
            0x81 => Some(Action::GotoFrame),
            0x8a => Some(Action::WaitForFrame),
            0x8b => Some(Action::SetTarget),
            0x8c => Some(Action::GotoLabel),
            0x8d => Some(Action::WaitForFrame2),
            0x96 => Some(Action::Push),
            0x99 => Some(Action::Jump),
            0x9a => Some(Action::GetUrl2),
            0x9d => Some(Action::If),
            0x9e => Some(Action::Call),
            0x9f => Some(Action::GotoFrame2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_opcodes_roundtrip() {
        // Every defined opcode should map to Some
        let cases: &[(u32, Action)] = &[
            (0x00, Action::End),
            (0x04, Action::NextFrame),
            (0x05, Action::PreviousFrame),
            (0x06, Action::Play),
            (0x07, Action::Stop),
            (0x09, Action::StopSounds),
            (0x0a, Action::Add),
            (0x0b, Action::Subtract),
            (0x0c, Action::Multiply),
            (0x0d, Action::Divide),
            (0x0e, Action::Equals),
            (0x0f, Action::Less),
            (0x10, Action::And),
            (0x11, Action::Or),
            (0x12, Action::Not),
            (0x13, Action::StringEquals),
            (0x14, Action::StringLength),
            (0x15, Action::StringExtract),
            (0x17, Action::Pop),
            (0x18, Action::ToInteger),
            (0x1c, Action::GetVariable),
            (0x1d, Action::SetVariable),
            (0x20, Action::SetTarget2),
            (0x21, Action::StringAdd),
            (0x22, Action::GetProperty),
            (0x23, Action::SetProperty),
            (0x24, Action::CloneSprite),
            (0x25, Action::RemoveSprite),
            (0x26, Action::Trace),
            (0x27, Action::StartDrag),
            (0x28, Action::EndDrag),
            (0x29, Action::StringLess),
            (0x30, Action::RandomNumber),
            (0x31, Action::MBStringLength),
            (0x32, Action::CharToAscii),
            (0x33, Action::AsciiToChar),
            (0x34, Action::GetTime),
            (0x35, Action::MBStringExtract),
            (0x36, Action::MBCharToAscii),
            (0x37, Action::MBAsciiToChar),
            (0x81, Action::GotoFrame),
            (0x8a, Action::WaitForFrame),
            (0x8b, Action::SetTarget),
            (0x8c, Action::GotoLabel),
            (0x8d, Action::WaitForFrame2),
            (0x96, Action::Push),
            (0x99, Action::Jump),
            (0x9a, Action::GetUrl2),
            (0x9d, Action::If),
            (0x9e, Action::Call),
            (0x9f, Action::GotoFrame2),
        ];
        for &(val, expected) in cases {
            assert_eq!(
                Action::from_u32(val),
                Some(expected),
                "opcode 0x{:02x}",
                val
            );
        }
    }

    #[test]
    fn test_unknown_opcodes_return_none() {
        for val in [
            0x01, 0x02, 0x03, 0x08, 0x16, 0x19, 0x1a, 0x1b, 0x1e, 0x1f, 0x2a, 0x80, 0xff,
        ] {
            assert_eq!(
                Action::from_u32(val),
                None,
                "expected None for 0x{:02x}",
                val
            );
        }
    }

    #[test]
    fn test_opcode_values_match_repr() {
        // Verify the discriminant values match what from_u32 expects
        assert_eq!(Action::End as u32, 0x00);
        assert_eq!(Action::Push as u32, 0x96);
        assert_eq!(Action::Add as u32, 0x0a);
        assert_eq!(Action::If as u32, 0x9d);
    }
}
