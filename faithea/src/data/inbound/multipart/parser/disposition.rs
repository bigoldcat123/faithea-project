const INVALID_PARAMETER: &str = "invalid multipart Content-Disposition parameter";

/// Parse parameters without slicing unchecked byte offsets or splitting quoted semicolons.
pub(super) fn parameters(value: &str) -> Result<Vec<(&str, String)>, &'static str> {
    let Some((_, mut rest)) = value.split_once(';') else {
        return Ok(Vec::new());
    };
    let mut parameters = Vec::new();
    loop {
        rest = rest.trim_start_matches([' ', '\t']);
        let (name, value) = rest.split_once('=').ok_or(INVALID_PARAMETER)?;
        let name = name.trim_end_matches([' ', '\t']);
        if name.is_empty() || !name.bytes().all(is_token_byte) {
            return Err(INVALID_PARAMETER);
        }
        let value = value.trim_start_matches([' ', '\t']);
        let (parsed, remaining) = if let Some(quoted) = value.strip_prefix('"') {
            let mut parsed = String::new();
            let mut escaped = false;
            let mut remaining = None;
            for (index, character) in quoted.char_indices() {
                if character.is_control() && character != '\t' {
                    return Err(INVALID_PARAMETER);
                }
                if escaped {
                    parsed.push(character);
                    escaped = false;
                } else {
                    match character {
                        '\\' => escaped = true,
                        '"' => {
                            remaining = Some(&quoted[index + 1..]);
                            break;
                        }
                        _ => parsed.push(character),
                    }
                }
            }
            (parsed, remaining.ok_or(INVALID_PARAMETER)?)
        } else {
            let token = value.split_once(';').map_or(value, |(token, _)| token);
            let token = token.trim_end_matches([' ', '\t']);
            if token.is_empty() || !token.bytes().all(is_token_byte) {
                return Err(INVALID_PARAMETER);
            }
            // Leave the separator for the shared validation below.
            (token.to_string(), &value[token.len()..])
        };
        parameters.push((name, parsed));
        let remaining = remaining.trim_start_matches([' ', '\t']);
        if remaining.is_empty() {
            return Ok(parameters);
        }
        rest = remaining.strip_prefix(';').ok_or(INVALID_PARAMETER)?;
    }
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_values_return_errors() {
        for value in [
            "",
            "\"",
            "\"unfinished",
            "\"a",
            "a\"",
            "中",
            "\"a\"junk",
            "\"a\\\"",
            "a b",
            "\"a\nb\"",
        ] {
            for name in ["name", "filename"] {
                assert!(
                    parameters(&format!("form-data; {name}={value}")).is_err(),
                    "accepted {name}={value:?}"
                );
            }
        }
    }

    #[test]
    fn quoted_and_token_values_are_preserved() {
        for (value, expected) in [
            ("a", "a"),
            ("abc", "abc"),
            ("\"\"", ""),
            ("\"a\"", "a"),
            ("\"中文😀\"", "中文😀"),
            ("\"a;b\"", "a;b"),
            ("\"a\\\"b\"", "a\"b"),
            ("\"a\\\\b\"", "a\\b"),
        ] {
            assert_eq!(
                parameters(&format!("form-data; name = {value} ; filename=sample.txt")).unwrap(),
                vec![
                    ("name", expected.to_string()),
                    ("filename", "sample.txt".to_string())
                ]
            );
        }
    }
}
