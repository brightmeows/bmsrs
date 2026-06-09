//! Parser for `#[bms_token("...")]` attribute strings.
//!
//! Supported template patterns:
//! - `"#TITLE {value}"`        — non-indexed with value
//! - `"#BPM{id} {value}"`      — indexed with id + value
//! - `"#ELSE"`                  — valueless
//! - `"%URL {value}"`           — non-indexed with `%` prefix
//! - `"#TEXT {value}"` (alias)  — multiple attrs on same variant
//! - `"#BASE 62"`               — non-indexed with literal value (no placeholder)

use std::fmt;

/// Parsed representation of a single `#[bms_token("...")]` attribute.
#[derive(Debug, Clone)]
pub struct BmsTokenTemplate {
    /// Prefix character (`#` or `%`).
    pub prefix: char,
    /// Command name, uppercased (e.g., `"TITLE"`, `"BPM"`, `"URL"`).
    pub command: String,
    /// If `Some`, this is an indexed command (e.g., `#BPM{id}`) and the value
    /// is the expected field name for the index (typically `"id"`).
    pub id_field: Option<String>,
    /// If `Some`, this command carries a placeholder value and the value is
    /// the expected field name (e.g., `"value"`, `"filename"`).
    pub value_field: Option<String>,
    /// If `Some`, this command carries a fixed literal value (e.g., `"62"` in
    /// `#BASE 62`). Mutually exclusive with `value_field`.
    pub value_literal: Option<String>,
}

impl BmsTokenTemplate {
    /// `true` if this command has an index placeholder in the command part.
    #[must_use]
    pub fn is_indexed(&self) -> bool {
        self.id_field.is_some()
    }

    /// `true` if this command expects a value (placeholder or literal).
    #[must_use]
    pub fn has_value(&self) -> bool {
        self.value_field.is_some() || self.value_literal.is_some()
    }

    /// `true` if this command uses a fixed literal value.
    #[cfg(test)]
    #[must_use]
    pub fn is_literal_value(&self) -> bool {
        self.value_literal.is_some()
    }
}

/// Errors that can occur when parsing a template string.
#[derive(Debug)]
pub struct TemplateParseError {
    /// A human-readable description of what went wrong.
    pub message: &'static str,
}

impl fmt::Display for TemplateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Parse a single `#[bms_token("...")]` attribute into a `BmsTokenTemplate`.
///
/// # Errors
///
/// Returns a `syn::Error` if the attribute content cannot be parsed as a
/// valid template string.
pub fn parse_bms_token_attr(attr: &syn::Attribute) -> syn::Result<BmsTokenTemplate> {
    let lit: syn::LitStr = attr.parse_args()?;
    parse_template_str(&lit.value()).map_err(|e| syn::Error::new_spanned(attr, e))
}

/// Parse a raw template string into a `BmsTokenTemplate`.
///
/// This is the single source of truth for template parsing — both the derive
/// macro and the build script call this function, ensuring they never diverge.
///
/// # Errors
///
/// Returns `TemplateParseError` if the string does not follow the expected
/// template format.
pub fn parse_template_str(s: &str) -> Result<BmsTokenTemplate, TemplateParseError> {
    let s = s.trim();

    let prefix = match s.chars().next() {
        Some(c @ ('#' | '%')) => c,
        Some(_) => {
            return Err(TemplateParseError {
                message: "template must start with '#' or '%'",
            });
        }
        None => {
            return Err(TemplateParseError {
                message: "template string is empty",
            });
        }
    };

    let after_prefix = &s[prefix.len_utf8()..];
    let rest = after_prefix.trim();
    if rest.is_empty() {
        return Err(TemplateParseError {
            message: "template has no command name after prefix",
        });
    }

    // Split at first space to separate command part from value part.
    let split_pos = rest.find(char::is_whitespace);
    let (command_part, value_part) = match split_pos {
        Some(pos) => (&rest[..pos], rest[pos..].trim()),
        None => (rest, ""),
    };

    let has_value = !value_part.is_empty();

    // Extract command base and optional id placeholder from command part.
    let (command, id_field) = extract_command_and_id(command_part)?;

    // Extract value field placeholder or literal from value part.
    let (value_field, value_literal) = if has_value {
        match extract_value_part(value_part) {
            Ok(ValuePart::Placeholder(name)) => (Some(name), None),
            Ok(ValuePart::Literal(lit)) => (None, Some(lit)),
            Err(e) => return Err(e),
        }
    } else {
        (None, None)
    };

    Ok(BmsTokenTemplate {
        prefix,
        command: command.to_uppercase(),
        id_field,
        value_field,
        value_literal,
    })
}

/// Extract the command name and optional `{id}` placeholder from the command
/// portion of a template.
///
/// `"TITLE"` → `("TITLE", None)`
/// `"BPM{id}"` → `("BPM", Some("id"))`
///
/// # Errors
///
/// Returns `TemplateParseError` if the braces are malformed.
fn extract_command_and_id(part: &str) -> Result<(String, Option<String>), TemplateParseError> {
    if let Some(open) = part.find('{') {
        if !part.ends_with('}') {
            return Err(TemplateParseError {
                message: "unclosed '{' in command part",
            });
        }
        let cmd = &part[..open];
        let id_name = &part[open + 1..part.len() - 1];
        if cmd.is_empty() {
            return Err(TemplateParseError {
                message: "command name is empty before '{'",
            });
        }
        if id_name.is_empty() {
            return Err(TemplateParseError {
                message: "placeholder name in command part is empty",
            });
        }
        Ok((cmd.to_owned(), Some(id_name.to_owned())))
    } else {
        Ok((part.to_owned(), None))
    }
}

/// The decoded value part of a template.
enum ValuePart {
    /// A placeholder like `{value}` or `{filename}`.
    Placeholder(String),
    /// A literal string like `62` in `#BASE 62`.
    Literal(String),
}

/// Extract the placeholder name or literal from the value part of a template.
///
/// `"{value}"` → `ValuePart::Placeholder("value")`
/// `"62"` → `ValuePart::Literal("62")`
///
/// # Errors
///
/// Returns `TemplateParseError` if the value part is empty.
fn extract_value_part(part: &str) -> Result<ValuePart, TemplateParseError> {
    let trimmed = part.trim();
    if trimmed.is_empty() {
        return Err(TemplateParseError {
            message: "value part is empty",
        });
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 3 {
        let name = &trimmed[1..trimmed.len() - 1];
        if name.is_empty() {
            return Err(TemplateParseError {
                message: "placeholder name in value part is empty",
            });
        }
        Ok(ValuePart::Placeholder(name.to_owned()))
    } else {
        Ok(ValuePart::Literal(trimmed.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_indexed_with_value() {
        let tmpl = parse_template_str("#TITLE {value}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "TITLE");
        assert!(!tmpl.is_indexed());
        assert_eq!(tmpl.value_field.as_deref(), Some("value"));
        assert!(tmpl.value_literal.is_none());
        assert!(!tmpl.is_literal_value());
    }

    #[test]
    fn indexed_with_id_and_value() {
        let tmpl = parse_template_str("#BPM{id} {value}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "BPM");
        assert!(tmpl.is_indexed());
        assert_eq!(tmpl.id_field.as_deref(), Some("id"));
        assert_eq!(tmpl.value_field.as_deref(), Some("value"));
    }

    #[test]
    fn valueless() {
        let tmpl = parse_template_str("#ELSE").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "ELSE");
        assert!(!tmpl.is_indexed());
        assert!(tmpl.value_field.is_none());
        assert!(tmpl.value_literal.is_none());
        assert!(!tmpl.has_value());
    }

    #[test]
    fn percent_prefix() {
        let tmpl = parse_template_str("%URL {value}").unwrap();
        assert_eq!(tmpl.prefix, '%');
        assert_eq!(tmpl.command, "URL");
        assert!(tmpl.value_field.is_some());
    }

    #[test]
    fn indexed_with_filename() {
        let tmpl = parse_template_str("#EXBMP{id} {filename}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "EXBMP");
        assert!(tmpl.is_indexed());
        assert_eq!(tmpl.id_field.as_deref(), Some("id"));
        assert_eq!(tmpl.value_field.as_deref(), Some("filename"));
    }

    #[test]
    fn literal_value() {
        let tmpl = parse_template_str("#BASE 62").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "BASE");
        assert!(!tmpl.is_indexed());
        assert!(tmpl.value_field.is_none());
        assert_eq!(tmpl.value_literal.as_deref(), Some("62"));
        assert!(tmpl.is_literal_value());
        assert!(tmpl.has_value());
    }

    #[test]
    fn empty_template_errors() {
        assert!(parse_template_str("").is_err());
    }

    #[test]
    fn missing_prefix_errors() {
        assert!(parse_template_str("TITLE {value}").is_err());
    }

    #[test]
    fn unclosed_brace_errors() {
        assert!(parse_template_str("#BPM{id {value}").is_err());
    }
}
