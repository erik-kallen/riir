use super::{is_valid_label, unresolved_instruction::UnresolvedInstruction, ParseErrorKind};

#[derive(Debug, PartialEq)]
pub(super) enum ParsedLineInstruction<'a> {
    Some(UnresolvedInstruction<'a>),
    None,
    Err(ParseErrorKind),
}

#[derive(Debug, PartialEq)]
pub(super) struct ParsedLine<'a> {
    pub labels: Vec<&'a str>,
    pub instruction: ParsedLineInstruction<'a>,
}

pub(super) fn parse_line<'a>(tokens: &[&'a str]) -> ParsedLine<'a> {
    let mut labels = Vec::<&str>::default();

    for i in 0..tokens.len() {
        let token = tokens[i];
        if token.ends_with(':') && is_valid_label(&token[0..token.len() - 1]) {
            labels.push(&token[0..token.len() - 1]);
        } else {
            return ParsedLine {
                labels,
                instruction: match UnresolvedInstruction::parse(&tokens[i..]) {
                    Ok(i) => ParsedLineInstruction::Some(i),
                    Err(e) => ParsedLineInstruction::Err(e),
                },
            };
        }
    }

    ParsedLine {
        labels,
        instruction: ParsedLineInstruction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::unresolved_instruction::{UnresolvedInstruction, UnresolvedSource};
    use super::{ParsedLineInstruction::*, *};
    use crate::instruction::{Register, Target};

    fn run(tokens: &[&str], expected_labels: &[&str], expected_instruction: ParsedLineInstruction) {
        assert_eq!(
            parse_line(tokens),
            ParsedLine {
                labels: expected_labels.to_owned(),
                instruction: expected_instruction
            }
        );
    }

    #[test]
    fn can_parse_empty_line() {
        run(&[], &[], None);
    }

    #[test]
    fn can_parse_line_with_only_instruction() {
        run(&["nop"], &[], Some(UnresolvedInstruction::Nop));
    }

    #[test]
    fn can_parse_line_with_instruction_and_operands() {
        run(
            &["inc", "eax"],
            &[],
            Some(UnresolvedInstruction::Inc(Target::Register(Register::Eax))),
        );
        run(
            &["add", "ebx", "1"],
            &[],
            Some(UnresolvedInstruction::Add(
                Target::Register(Register::Ebx),
                UnresolvedSource::Value(1),
            )),
        );
        run(
            &["jmp", "label"],
            &[],
            Some(UnresolvedInstruction::Jmp(UnresolvedSource::Label("label"))),
        );
    }

    #[test]
    fn can_parse_line_with_only_labels() {
        run(&["label1:"], &["label1"], None);
        run(&["label1:", "label2:"], &["label1", "label2"], None);
    }

    #[test]
    fn can_parse_line_with_labels_and_instruction() {
        run(
            &["label1:", "nop"],
            &["label1"],
            Some(UnresolvedInstruction::Nop),
        );
        run(
            &["label1:", "label2:", "nop"],
            &["label1", "label2"],
            Some(UnresolvedInstruction::Nop),
        );

        run(
            &["label1:", "inc", "eax"],
            &["label1"],
            Some(UnresolvedInstruction::Inc(Target::Register(Register::Eax))),
        );
        run(
            &["label1:", "label2:", "inc", "eax"],
            &["label1", "label2"],
            Some(UnresolvedInstruction::Inc(Target::Register(Register::Eax))),
        );
    }

    #[test]
    fn errors_are_correctly_reported() {
        run(
            &["push", "label1:"],
            &[],
            Err(ParseErrorKind::InvalidOperand("label1:".to_owned())),
        );
        run(
            &["pop", "label1:"],
            &[],
            Err(ParseErrorKind::InvalidOperand("label1:".to_owned())),
        );
        run(
            &["bad"],
            &[],
            Err(ParseErrorKind::InvalidInstruction("bad".to_owned())),
        );
        run(&["add", "eax"], &[], Err(ParseErrorKind::MissingOperand(2)));
        run(
            &["nop", "eax"],
            &[],
            Err(ParseErrorKind::ExtraToken("eax".to_owned())),
        );
        run(
            &["inc", "eax", "ebx"],
            &[],
            Err(ParseErrorKind::ExtraToken("ebx".to_owned())),
        );
    }

    #[test]
    fn labels_are_returned_for_lines_with_errors() {
        run(
            &["label1:", "bad"],
            &["label1"],
            Err(ParseErrorKind::InvalidInstruction("bad".to_owned())),
        );
    }

    #[test]
    fn garbage_with_colon_is_not_parsed_as_label() {
        run(
            &["wef(#):", "inc", "eax"],
            &[],
            Err(ParseErrorKind::InvalidInstruction("wef(#):".to_owned())),
        );
    }
}
