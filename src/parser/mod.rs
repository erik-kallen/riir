mod line_parser;
pub mod old_interface;
mod register;
mod resolver;
mod unresolved_instruction;

fn is_valid_label(s: &str) -> bool {
    fn is_valid_first_char(c: u8) -> bool {
        match c {
            b'$' | b'@' | b'_' => true,
            b'A'..=b'Z' => true,
            b'a'..=b'z' => true,
            _ => false,
        }
    }

    fn is_valid_char(c: u8) -> bool {
        match c {
            b'0'..=b'9' => true,
            _ => is_valid_first_char(c),
        }
    }

    let mut bytes = s.bytes();
    if let Some(first) = bytes.next() {
        is_valid_first_char(first) && bytes.all(is_valid_char)
    } else {
        false
    }
}
