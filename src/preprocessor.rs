use crate::htab::{HashTable, Item};
use std::{
    collections::hash_map::Entry,
    ffi::CString,
    io::Error,
    os::raw::{c_int, c_void},
    str,
};

pub const TOK_INCLUDE: &str = "%include";
pub const TOK_DEFINE: &str = "%define";

#[no_mangle]
pub(crate) unsafe extern "C" fn tvm_preprocess(
    src: *mut *mut ::std::os::raw::c_char,
    src_len: *mut ::std::os::raw::c_int,
    defines: *mut c_void,
) -> c_int {
    if src.is_null() || (*src).is_null() || src_len.is_null() || defines.is_null() {
        return -1;
    }

    // Safety: This assumes the tvm_htab_ctx is actually our ported HashTable
    let defines = &mut *(defines as *mut HashTable);

    // convert the input string to an owned Rust string so it can be
    // preprocessed
    let rust_src = match CString::from_raw(*src).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -1,
    };

    match preprocess(rust_src, defines) {
        Ok(s) => {
            *src_len = s.len() as c_int;
            *src = CString::new(s).unwrap().into_raw();

            // returning 0 indicates success
            0
        }
        Err(_) => {
            // tell the caller "an error occurred"
            *src_len = 0;
            *src = CString::new("").unwrap().into_raw();

            -1
        }
    }
}

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
    defines: &mut HashTable,
) -> Result<(String, bool), PreprocessingError> {
    process_directive_line(src, TOK_DEFINE, |line| {
        parse_define(line, defines)?;
        Ok(String::from("\n"))
    })
}

fn parse_define(line: &str, defines: &mut HashTable) -> Result<(), PreprocessingError> {
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

    match defines
        .0
        .entry(CString::new(key).expect("The text shouldn't contain null bytes"))
    {
        Entry::Vacant(vacant) => vacant.insert(Item::opaque(CString::new(value).unwrap())),
        Entry::Occupied(occupied) => {
            return Err(PreprocessingError::DuplicateDefine {
                name: key.to_string(),
                original_value: occupied
                    .get()
                    .opaque_value_str()
                    .unwrap_or("<invalid>")
                    .to_string(),
                new_value: value.to_string(),
            });
        }
    };

    Ok(())
}

pub fn preprocess(mut src: String, defines: &mut HashTable) -> Result<String, PreprocessingError> {
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
    use crate::htab::{tvm_htab_create, tvm_htab_destroy, tvm_htab_find_ref, HashTable};
    use std::{
        ffi::{CStr, CString},
        io::Write,
        os::raw::c_int,
    };

    #[test]
    fn find_all_defines() {
        let src = "%define true 1\nsome random text\n%define FOO_BAR -42\n";
        let original_length = src.len();
        let mut src = CString::new(src).unwrap().into_raw();

        unsafe {
            let mut len = original_length as c_int;
            let defines = tvm_htab_create();

            let ret = tvm_preprocess(&mut src, &mut len, defines);

            assert_eq!(ret, 0);

            let preprocessed = CString::from_raw(src);
            let preprocessed = preprocessed.to_str().unwrap();

            assert_eq!(preprocessed, "\nsome random text\n\n");

            let true_define = tvm_htab_find_ref(defines, b"true\0".as_ptr().cast());
            assert_ne!(true_define, std::ptr::null());
            let got = CStr::from_ptr(true_define).to_str().unwrap();
            assert_eq!(got, "1");

            let foo_bar = tvm_htab_find_ref(defines, b"FOO_BAR\0".as_ptr().cast());
            assert_ne!(foo_bar, std::ptr::null());
            let got = CStr::from_ptr(foo_bar).to_str().unwrap();
            assert_eq!(got, "-42");

            tvm_htab_destroy(defines);
        }
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

        unsafe {
            let mut len = top_level_src.len() as c_int;
            let top_level_src = CString::new(top_level_src).unwrap();
            let mut src = top_level_src.into_raw();
            let defines = tvm_htab_create();

            let ret = tvm_preprocess(&mut src, &mut len, defines);

            assert_eq!(ret, 0);

            let preprocessed = CString::from_raw(src).into_string().unwrap();

            assert_eq!(
                preprocessed,
                "first line\nfirst nested\nreally nested\nlast nested\nlast line\n"
            );

            tvm_htab_destroy(defines);
        }
    }

    #[test]
    fn empty_string() {
        let src = String::from("");
        let mut hashtable = HashTable::default();

        let (got, replacements) = process_defines(src, &mut hashtable).unwrap();

        assert!(got.is_empty());
        assert_eq!(replacements, false);
        assert!(hashtable.0.is_empty());
    }

    #[test]
    fn false_percent() {
        let src = String::from("this string contains a % symbol");
        let mut hashtable = HashTable::default();

        let (got, replacements) = process_defines(src.clone(), &mut hashtable).unwrap();

        assert_eq!(got, src);
        assert_eq!(replacements, false);
        assert!(hashtable.0.is_empty());
    }

    #[test]
    fn define_without_key_and_value() {
        let src = String::from("%define\n");
        let mut hashtable = HashTable::default();

        let err = process_defines(src.clone(), &mut hashtable).unwrap_err();

        match err {
            PreprocessingError::EmptyDefine => {}
            other => panic!("Expected EmptyDefine, found {:?}", other),
        }
    }

    #[test]
    fn define_without_value() {
        let src = String::from("%define key\n");
        let mut hashtable = HashTable::default();

        let err = process_defines(src.clone(), &mut hashtable).unwrap_err();

        match err {
            PreprocessingError::DefineWithoutValue(key) => assert_eq!(key, "key"),
            other => panic!("Expected DefineWithoutValue, found {:?}", other),
        }
    }

    #[test]
    fn valid_define() {
        let src = String::from("%define key value\n");
        let mut hashtable = HashTable::default();

        let (got, had_defines) = process_defines(src.clone(), &mut hashtable).unwrap();

        assert_eq!(got, "\n");
        assert_eq!(had_defines, true);
        assert_eq!(hashtable.0.len(), 1);
        let key = CString::new("key").unwrap();
        let item = hashtable.0.get(&key).unwrap();
        assert_eq!(item.opaque_value_str().unwrap(), "value");
    }
}
