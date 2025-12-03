//! Regex-based extraction of cipher functions from YouTube player.js.
//!
//! YouTube's player JavaScript contains two critical functions:
//! 1. Signature decipher - transforms encrypted 's' parameter into valid signature
//! 2. N-transform - transforms 'n' parameter to bypass throttling

use crate::{Error, Result};
use regex::Regex;

/// Extracted cipher functions from YouTube's player.js.
#[derive(Debug, Clone)]
pub struct CipherFunctions {
    /// The signature decipher function and its helper object.
    pub decipher_fn: String,
    /// The n-transform function.
    pub n_transform_fn: String,
}

impl CipherFunctions {
    /// Extract cipher functions from player.js source code.
    pub fn extract(player_js: &str) -> Result<Self> {
        let decipher_fn = extract_decipher_function(player_js)?;
        let n_transform_fn = extract_n_transform_function(player_js)?;

        Ok(Self {
            decipher_fn,
            n_transform_fn,
        })
    }
}

/// Extract the signature decipher function.
fn extract_decipher_function(player_js: &str) -> Result<String> {
    // Find the decipher function by looking for the characteristic pattern:
    // a=a.split("") ... return a.join("")
    let fn_pattern = Regex::new(
        r#"([a-zA-Z0-9$_]+)\s*=\s*function\s*\(\s*([a-zA-Z])\s*\)\s*\{\s*[a-zA-Z]\s*=\s*[a-zA-Z]\.split\s*\(\s*""\s*\)\s*;([^}]+?return\s+[a-zA-Z]\.join\s*\(\s*""\s*\))\s*\}"#
    ).map_err(|e| Error::Parse(e.to_string()))?;

    let (_fn_name, fn_body) = fn_pattern
        .captures(player_js)
        .map(|caps| {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("decipher");
            let arg = caps.get(2).map(|m| m.as_str()).unwrap_or("a");
            let body = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let full_fn = format!("function {}({}){{{1}={1}.split(\"\");{}}}", name, arg, body);
            (name.to_string(), full_fn)
        })
        .ok_or_else(|| Error::Parse("Could not find decipher function".to_string()))?;

    // Extract the helper object name from the function body
    let helper_re = Regex::new(r"([a-zA-Z0-9$_]+)\.[a-zA-Z0-9$_]+\([a-zA-Z],")
        .map_err(|e| Error::Parse(e.to_string()))?;

    let helper_obj = if let Some(caps) = helper_re.captures(&fn_body) {
        if let Some(name) = caps.get(1) {
            extract_helper_object(player_js, name.as_str())?
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Ok(format!("{}\n{}", helper_obj, fn_body))
}

/// Extract the helper object that contains cipher operations.
fn extract_helper_object(player_js: &str, name: &str) -> Result<String> {
    let start_pattern = format!(r"var\s+{}\s*=\s*\{{", regex::escape(name));
    let start_re = Regex::new(&start_pattern).map_err(|e| Error::Parse(e.to_string()))?;

    if let Some(m) = start_re.find(player_js) {
        let start_idx = m.start();
        let after_start = &player_js[m.end()..];

        let end_offset = find_matching_brace_simple(after_start);

        if end_offset > 0 {
            let end_idx = m.end() + end_offset;
            let result = if player_js[end_idx..].starts_with(';') {
                &player_js[start_idx..=end_idx]
            } else {
                &player_js[start_idx..end_idx]
            };
            return Ok(result.to_string());
        }
    }

    Ok(String::new())
}

/// Extract the n-transform function (throttling bypass).
fn extract_n_transform_function(player_js: &str) -> Result<String> {
    let n_ref_patterns = [
        r#"\.get\("n"\)\)&&\(b=([a-zA-Z0-9$_]+)(?:\[(\d+)\])?\(b\)"#,
        r#"&&\(b=([a-zA-Z0-9$_]+)\[(\d+)\]\(b\)"#,
        r#"c=([a-zA-Z0-9$_]+)\(decodeURIComponent"#,
    ];

    let mut var_name = None;
    let mut array_idx = None;

    for pattern in &n_ref_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(player_js) {
                var_name = caps.get(1).map(|m| m.as_str().to_string());
                array_idx = caps.get(2).and_then(|m| m.as_str().parse::<usize>().ok());
                if var_name.is_some() {
                    break;
                }
            }
        }
    }

    let var_name =
        var_name.ok_or_else(|| Error::Parse("Could not find n-transform reference".to_string()))?;

    // If it's an array reference, resolve the actual function name
    if array_idx.is_some() {
        let array_pattern = format!(r"var\s+{}\s*=\s*\[([^\]]+)\]", regex::escape(&var_name));
        if let Ok(re) = Regex::new(&array_pattern) {
            if let Some(caps) = re.captures(player_js) {
                if let Some(array_content) = caps.get(1) {
                    let items: Vec<&str> = array_content.as_str().split(',').collect();
                    if let Some(idx) = array_idx {
                        if let Some(fn_name) = items.get(idx) {
                            let fn_name = fn_name.trim();
                            return extract_function_with_deps(player_js, fn_name);
                        }
                    }
                }
            }
        }
    }

    extract_function_with_deps(player_js, &var_name)
}

/// Extract a function and any dependencies it needs.
fn extract_function_with_deps(player_js: &str, name: &str) -> Result<String> {
    let patterns = [
        // Pattern 1: var name=function(a){...}
        (
            format!(r"var\s+{}\s*=\s*function\s*\([^)]*\)", regex::escape(name)),
            false,
        ),
        // Pattern 2: function name(a){...}
        (
            format!(r"function\s+{}\s*\([^)]*\)", regex::escape(name)),
            false,
        ),
        // Pattern 3: name=function(a){...} (without var, after delimiter or whitespace)
        (
            format!(r"[,;\n\r\t ]{}=function\s*\([^)]*\)", regex::escape(name)),
            true,
        ),
    ];

    for (pattern, skip_first_char) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(m) = re.find(player_js) {
                let start_idx = if *skip_first_char {
                    m.start() + 1
                } else {
                    m.start()
                };

                let after_sig = &player_js[m.end()..];

                if let Some(brace_start) = after_sig.find('{') {
                    let fn_start = m.end() + brace_start;
                    let fn_body = &player_js[fn_start..];

                    let end_offset = find_matching_brace(fn_body);

                    if end_offset > 0 {
                        let end_idx = fn_start + end_offset;
                        let result = &player_js[start_idx..end_idx];
                        if result.ends_with('}') {
                            return Ok(format!("{};", result));
                        }
                        return Ok(result.to_string());
                    }
                }
            }
        }
    }

    Err(Error::Parse(format!(
        "Could not extract function: {}",
        name
    )))
}

/// Simple brace counting (for helper objects without nested strings).
fn find_matching_brace_simple(code: &str) -> usize {
    let mut brace_count = 1;

    for (i, c) in code.char_indices() {
        match c {
            '{' => brace_count += 1,
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }

    0
}

/// Find the matching closing brace, handling nested braces and strings.
fn find_matching_brace(code: &str) -> usize {
    let mut brace_count = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev_char = ' ';

    for (i, c) in code.char_indices() {
        if !in_string && (c == '"' || c == '\'') {
            in_string = true;
            string_char = c;
        } else if in_string && c == string_char && prev_char != '\\' {
            in_string = false;
        } else if !in_string {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
        prev_char = c;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_helper_object() {
        let js = r#"
            var someCode = 123;
            var Az={qn:function(a,b){a.splice(0,b)},m0:function(a,b){var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c},Bx:function(a){a.reverse()}};
            var moreCode = 456;
        "#;

        let result = extract_helper_object(js, "Az").unwrap();
        assert!(result.contains("qn:function"));
        assert!(result.contains("m0:function"));
        assert!(result.contains("Bx:function"));
    }

    #[test]
    fn test_extract_function_with_deps() {
        let js = r#"
            var iha=function(a){var b=a.split(""),c=[function(){return 1}];return b.join("")};
            var other = 123;
        "#;

        let result = extract_function_with_deps(js, "iha").unwrap();
        assert!(result.contains("function(a)"));
        assert!(result.contains("split"));
    }

    #[test]
    fn test_extract_function_without_var() {
        let js = r#"
            someOther=123;
            iha=function(a){var b=a.split("");return b.join("")};
            more=456;
        "#;

        let result = extract_function_with_deps(js, "iha").unwrap();
        assert!(result.contains("function(a)"));
        assert!(result.contains("split"));
    }
}
