use std::{collections::{HashMap, hash_map::Entry}, io::Error};

pub const TOK_INCLUDE: &str = "%include";
pub const TOK_DEFINE: &str = "%define";

#[derive(Debug)]
pub enum PreprocessingError {
    FailedInclude {
        name: String,
        inner: Error,
    },
    DuplicateDefine {
        name: String,
        original_value: String,
        new_value: String,
    },
    EmptyDefine,
    DefineWithoutValue(String),
}

/// Scan through the input string looking for a line starting with some
/// directive, using a callback to figure out what to replace the directive line
/// with.
fn process_directive_line<F>(
    mut src: String,
    directive: &str,
    replace_line: F,
) -> Result<(String, bool), PreprocessingError>
where
    F: FnOnce(&str) -> Result<String, PreprocessingError>,
{
    let directive_delimiter = match src.find(directive) {
        Some(ix) => ix,
        None => return Ok((src, false)),
    };

    let end_ix = src[directive_delimiter..]
        .find('\n')
        .map(|ix| ix + directive_delimiter)
        .unwrap_or(src.len());

    let directive_line = src[directive_delimiter + directive.len()..end_ix].trim();

    let replacement = replace_line(directive_line)?;

    src.drain(directive_delimiter..end_ix + 1);
    src.insert_str(directive_delimiter, &replacement);

    Ok((src, true))
}

fn process_includes(src: String) -> Result<(String, bool), PreprocessingError> {
    process_directive_line(src, TOK_INCLUDE, |line| {
        std::fs::read_to_string(line).map_err(|e| PreprocessingError::FailedInclude {
            name: line.to_string(),
            inner: e,
        })
    })
}

fn process_defines(
    src: String,
    defines: &mut HashMap<String, String>,
) -> Result<(String, bool), PreprocessingError> {
    process_directive_line(src, TOK_DEFINE, |line| {
        parse_define(line, defines)?;
        Ok(String::from("\n"))
    })
}

fn parse_define(line: &str, defines: &mut HashMap<String, String>) -> Result<(), PreprocessingError> {
    if line.is_empty() {
        return Err(PreprocessingError::EmptyDefine);
    }

    // The syntax is "%define key value", so after removing the leading
    // "%define" everything after the next space is the value
    let first_space = line
        .find(' ')
        .ok_or_else(|| PreprocessingError::DefineWithoutValue(line.to_string()))?;

    let (key, value) = line.split_at(first_space);
    let value = value.trim();

    match defines.entry(key.to_owned())
    {
        Entry::Vacant(vacant) => vacant.insert(value.to_owned()),
        Entry::Occupied(occupied) => {
            return Err(PreprocessingError::DuplicateDefine {
                name: key.to_string(),
                original_value: occupied.get().clone(),
                new_value: value.to_string(),
            });
        }
    };

    Ok(())
}

pub fn preprocess(mut src: String, defines: &mut HashMap<String, String>) -> Result<String, PreprocessingError> {
    loop {
        let (modified, any_includes) = process_includes(src)?;
        let (modified, any_defines) = process_defines(modified, defines)?;

        if !any_includes && !any_defines {
            return Ok(modified);
        }

        src = modified;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn find_all_defines() {
        let src = "%define true 1\nsome random text\n%define FOO_BAR -42\n".to_owned();

        let mut defines = HashMap::<String, String>::default();

        let preprocessed = preprocess(src, &mut defines).unwrap();

        assert_eq!(preprocessed, "\nsome random text\n\n");

        assert_eq!(defines.get("true").unwrap(), "1");
        assert_eq!(defines.get("FOO_BAR").unwrap(), "-42");
    }

    #[test]
    fn include_another_file() {
        const TOP_LEVEL: &str = "first line\n%include nested\nlast line\n";
        const NESTED: &str = "first nested\n%include reallynested\nlast nested\n";
        const REALLY_NESTED: &str = "really nested\n";

        // Write the really nested file
        let mut really_nested = tempfile::NamedTempFile::new().unwrap();
        really_nested.write_all(REALLY_NESTED.as_bytes()).unwrap();
        let really_nested_filename = really_nested.path().to_str().unwrap();

        // Substitute the full path to the really nested file
        let nested_src = NESTED.replace("reallynested", really_nested_filename);

        // Write the nested file
        let mut nested = tempfile::NamedTempFile::new().unwrap();
        nested.write_all(nested_src.as_bytes()).unwrap();
        let nested_filename = nested.path().to_str().unwrap();

        // Substitute the full path to the nested file
        let top_level_src = TOP_LEVEL.replace("nested", nested_filename);

        let mut defines = HashMap::<String, String>::default();

        let preprocessed = preprocess(top_level_src, &mut defines).unwrap();

        assert_eq!(
            preprocessed,
            "first line\nfirst nested\nreally nested\nlast nested\nlast line\n"
        );
    }

    #[test]
    fn empty_string() {
        let src = String::from("");
        let mut defines = HashMap::<String, String>::default();

        let (got, replacements) = process_defines(src, &mut defines).unwrap();

        assert!(got.is_empty());
        assert_eq!(replacements, false);
        assert!(defines.is_empty());
    }

    #[test]
    fn false_percent() {
        let src = String::from("this string contains a % symbol");
        let mut defines = HashMap::<String, String>::default();

        let (got, replacements) = process_defines(src.clone(), &mut defines).unwrap();

        assert_eq!(got, src);
        assert_eq!(replacements, false);
        assert!(defines.is_empty());
    }

    #[test]
    fn define_without_key_and_value() {
        let src = String::from("%define\n");
        let mut defines = HashMap::<String, String>::default();

        let err = process_defines(src.clone(), &mut defines).unwrap_err();

        match err {
            PreprocessingError::EmptyDefine => {}
            other => panic!("Expected EmptyDefine, found {:?}", other),
        }
    }

    #[test]
    fn define_without_value() {
        let src = String::from("%define key\n");
        let mut defines = HashMap::<String, String>::default();

        let err = process_defines(src.clone(), &mut defines).unwrap_err();

        match err {
            PreprocessingError::DefineWithoutValue(key) => assert_eq!(key, "key"),
            other => panic!("Expected DefineWithoutValue, found {:?}", other),
        }
    }

    #[test]
    fn valid_define() {
        let src = String::from("%define key value\n");
        let mut defines = HashMap::<String, String>::default();

        let (got, had_defines) = process_defines(src.clone(), &mut defines).unwrap();

        assert_eq!(got, "\n");
        assert_eq!(had_defines, true);
        assert_eq!(defines.len(), 1);
        assert_eq!(defines.get("key").unwrap(), "value");
    }
}
