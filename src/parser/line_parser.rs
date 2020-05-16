use super::{
    is_valid_label,
    unresolved_instruction::{ParseInstructionError, UnresolvedInstruction},
};

#[derive(Debug, PartialEq)]
pub(super) enum ParsedLineInstruction<'a> {
    Some(UnresolvedInstruction<'a>),
    None,
    Err(ParseInstructionError<'a>),
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
    use super::super::unresolved_instruction::{
        ParseInstructionError, UnresolvedInstruction, UnresolvedSource,
    };
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
            Err(ParseInstructionError::InvalidSource("label1:")),
        );
        run(
            &["pop", "label1:"],
            &[],
            Err(ParseInstructionError::InvalidTarget("label1:")),
        );
        run(
            &["bad"],
            &[],
            Err(ParseInstructionError::InvalidInstruction("bad")),
        );
        run(
            &["add", "eax"],
            &[],
            Err(ParseInstructionError::MissingOperand(2)),
        );
        run(
            &["nop", "eax"],
            &[],
            Err(ParseInstructionError::ExtraToken("eax")),
        );
        run(
            &["inc", "eax", "ebx"],
            &[],
            Err(ParseInstructionError::ExtraToken("ebx")),
        );
    }

    #[test]
    fn labels_are_returned_for_lines_with_errors() {
        run(
            &["label1:", "bad"],
            &["label1"],
            Err(ParseInstructionError::InvalidInstruction("bad")),
        );
    }

    #[test]
    fn garbage_with_colon_is_not_parsed_as_label() {
        run(
            &["wef(#):", "inc", "eax"],
            &[],
            Err(ParseInstructionError::InvalidInstruction("wef(#):")),
        );
    }
}
