//! `#[bms_token("...")]` 属性字符串的解析器。
//!
//! 支持的模板模式：
//! - `"#TITLE {}"` —— 非索引，匿名 value
//! - `"#TITLE {value}"` —— 非索引，命名 value
//! - `"#BPM{id} {value}"` —— 索引，带 id + value
//! - `"#BPM{} {}"` —— 索引，匿名 id + 匿名 value
//! - `"#ELSE"` —— 无 value
//! - `"%URL {}"` —— 非索引，带 `%` 前缀
//! - `"#TEXT {text}"`（别名） —— 同一变体上的多个属性
//! - `"#BASE 62"` —— 非索引，字面值（无占位符）

use std::fmt;

/// 模板中的占位符是命名还是匿名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placeholder {
    /// 命名占位符，如 `{value}`、`{id}` 或 `{filename}`。
    Named(String),
    /// 匿名占位符 `{}`，绑定到匿名字段位置。
    Unnamed,
}

impl Placeholder {
    /// 若为命名占位符，返回其名称。
    #[must_use]
    pub const fn name(&self) -> Option<&str> {
        match self {
            Self::Named(n) => Some(n.as_str()),
            Self::Unnamed => None,
        }
    }

    /// 当为命名占位符时返回 `true`。
    #[must_use]
    pub const fn is_named(&self) -> bool {
        matches!(self, Self::Named(_))
    }
}

/// 单个 `#[bms_token("...")]` 属性解析后的表示。
#[derive(Debug, Clone)]
pub struct BmsTokenTemplate {
    /// 前缀字符（`#` 或 `%`）。
    pub prefix: char,
    /// 命令名，已转为大写（如 `"TITLE"`、`"BPM"`、`"URL"`）。
    pub command: String,
    /// 若为 `Some`，表示这是一个索引命令（如 `#BPM{id}`），该值是索引期望
    /// 引用的字段。
    pub id_field: Option<Placeholder>,
    /// 若为 `Some`，表示该命令带占位符值（命名或匿名）。
    pub value_field: Option<Placeholder>,
    /// 若为 `Some`，表示该命令带固定字面值（如 `#BASE 62` 中的 `"62"`）。
    /// 与 `value_field` 互斥。
    pub value_literal: Option<String>,
}

impl BmsTokenTemplate {
    /// 当命令部分含有索引占位符时返回 `true`。
    #[must_use]
    pub const fn is_indexed(&self) -> bool {
        self.id_field.is_some()
    }

    /// 当该命令期望一个值（占位符或字面值）时返回 `true`。
    #[must_use]
    pub const fn has_value(&self) -> bool {
        self.value_field.is_some() || self.value_literal.is_some()
    }

    /// 当该命令使用固定字面值时返回 `true`。
    #[cfg(test)]
    #[must_use]
    pub const fn is_literal_value(&self) -> bool {
        self.value_literal.is_some()
    }

    /// 当命令部分的索引占位符为匿名（`{}`）时返回 `true`。
    #[must_use]
    pub fn is_unnamed_id(&self) -> bool {
        self.id_field.as_ref().is_some_and(|p| !p.is_named())
    }

    /// 当值部分的占位符为匿名（`{}`）时返回 `true`。
    #[must_use]
    pub fn is_unnamed_value(&self) -> bool {
        self.value_field.as_ref().is_some_and(|p| !p.is_named())
    }

    /// 返回 id 占位符的实际字段名。
    /// 命名占位符返回其名称；匿名占位符返回 `"id"`。
    #[must_use]
    pub fn id_field_name(&self) -> &str {
        self.id_field
            .as_ref()
            .and_then(|p| p.name())
            .unwrap_or("id")
    }

    /// 返回 value 占位符的实际字段名。
    /// 命名占位符返回其名称；匿名占位符返回 `"value"`。
    #[must_use]
    pub fn value_field_name(&self) -> &str {
        self.value_field
            .as_ref()
            .and_then(|p| p.name())
            .unwrap_or("value")
    }
}

/// 解析模板字符串时可能发生的错误。
#[derive(Debug)]
pub struct TemplateParseError {
    /// 出错原因的人类可读描述。
    pub message: &'static str,
}

impl fmt::Display for TemplateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// 将单个 `#[bms_token("...")]` 属性解析为 `BmsTokenTemplate`。
///
/// # Errors
///
/// 当属性内容无法解析为合法模板字符串时返回 `syn::Error`。
pub fn parse_bms_token_attr(attr: &syn::Attribute) -> syn::Result<BmsTokenTemplate> {
    let lit: syn::LitStr = attr.parse_args()?;
    parse_template_str(&lit.value()).map_err(|e| syn::Error::new_spanned(attr, e))
}

/// 将原始模板字符串解析为 `BmsTokenTemplate`。
///
/// 这是模板解析的唯一真源 —— derive 宏与构建脚本都调用此函数，
/// 确保两者不会产生分歧。
///
/// # Errors
///
/// 当字符串不符合期望的模板格式时返回 `TemplateParseError`。
#[expect(
    clippy::string_slice,
    reason = "BMS token templates are ASCII-only; indexing at byte boundaries is safe"
)]
pub fn parse_template_str(s: &str) -> Result<BmsTokenTemplate, TemplateParseError> {
    #[expect(clippy::shadow_reuse, reason = "intentional self-shadow to trim")]
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

    // 在第一个空格处分割，将命令部分与值部分分开。
    let split_pos = rest.find(char::is_whitespace);
    let (command_part, value_part) =
        split_pos.map_or((rest, ""), |pos| (&rest[..pos], rest[pos..].trim()));

    let has_value = !value_part.is_empty();

    // 从命令部分提取命令基名与可选的 id 占位符。
    let (command, id_field) = extract_command_and_id(command_part)?;

    // 从值部分提取 value 字段占位符或字面值。
    let (value_field, value_literal) = if has_value {
        match extract_value_part(value_part) {
            Ok(ValuePart::Placeholder(placeholder)) => (Some(placeholder), None),
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

/// 从模板的命令部分提取命令名与可选的 `{...}` 占位符。
///
/// `"TITLE"` → `("TITLE", None)`
/// `"BPM{id}"` → `("BPM", Some(Placeholder::Named("id")))`
/// `"BPM{}"` → `("BPM", Some(Placeholder::Unnamed))`
///
/// # Errors
///
/// 当花括号格式错误时返回 `TemplateParseError`。
#[expect(
    clippy::string_slice,
    reason = "BMS command parts are ASCII-only; indexing at byte boundaries is safe"
)]
fn extract_command_and_id(part: &str) -> Result<(String, Option<Placeholder>), TemplateParseError> {
    if let Some(open) = part.find('{') {
        if !part.ends_with('}') {
            return Err(TemplateParseError {
                message: "unclosed '{' in command part",
            });
        }
        let cmd = &part[..open];
        let id_content = &part[open + 1..part.len() - 1];
        if cmd.is_empty() {
            return Err(TemplateParseError {
                message: "command name is empty before '{'",
            });
        }
        let placeholder = if id_content.is_empty() {
            Placeholder::Unnamed
        } else {
            Placeholder::Named(id_content.to_owned())
        };
        Ok((cmd.to_owned(), Some(placeholder)))
    } else {
        Ok((part.to_owned(), None))
    }
}

/// 模板中值部分的解码结果。
enum ValuePart {
    /// 占位符，如 `{}`、`{value}` 或 `{filename}`。
    Placeholder(Placeholder),
    /// 字面字符串，如 `#BASE 62` 中的 `62`。
    Literal(String),
}

/// 从模板的值部分提取占位符或字面值。
///
/// `"{}"` → `ValuePart::Placeholder(Placeholder::Unnamed)`
/// `"{value}"` → `ValuePart::Placeholder(Placeholder::Named("value"))`
/// `"62"` → `ValuePart::Literal("62")`
///
/// # Errors
///
/// 当值部分为空时返回 `TemplateParseError`。
#[expect(
    clippy::string_slice,
    reason = "BMS token value parts are ASCII-only; indexing at byte boundaries is safe"
)]
fn extract_value_part(part: &str) -> Result<ValuePart, TemplateParseError> {
    let trimmed = part.trim();
    if trimmed.is_empty() {
        return Err(TemplateParseError {
            message: "value part is empty",
        });
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        let content = &trimmed[1..trimmed.len() - 1];
        let placeholder = if content.is_empty() {
            Placeholder::Unnamed
        } else {
            Placeholder::Named(content.to_owned())
        };
        Ok(ValuePart::Placeholder(placeholder))
    } else {
        Ok(ValuePart::Literal(trimmed.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_indexed_with_unnamed_value() {
        let tmpl = parse_template_str("#TITLE {}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "TITLE");
        assert!(!tmpl.is_indexed());
        assert!(tmpl.value_field.is_some());
        assert!(tmpl.is_unnamed_value());
        assert!(!tmpl.value_field.as_ref().unwrap().is_named());
        assert!(tmpl.value_literal.is_none());
        assert!(!tmpl.is_literal_value());
    }

    #[test]
    fn non_indexed_with_named_value() {
        let tmpl = parse_template_str("#TITLE {value}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "TITLE");
        assert!(!tmpl.is_indexed());
        assert_eq!(
            tmpl.value_field.as_ref().and_then(|p| p.name()),
            Some("value")
        );
        assert!(tmpl.value_literal.is_none());
    }

    #[test]
    fn indexed_with_id_and_value() {
        let tmpl = parse_template_str("#BPM{id} {value}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "BPM");
        assert!(tmpl.is_indexed());
        assert_eq!(tmpl.id_field.as_ref().and_then(|p| p.name()), Some("id"));
        assert_eq!(
            tmpl.value_field.as_ref().and_then(|p| p.name()),
            Some("value")
        );
    }

    #[test]
    fn indexed_with_unnamed_both() {
        let tmpl = parse_template_str("#BPM{} {}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "BPM");
        assert!(tmpl.is_indexed());
        assert!(tmpl.is_unnamed_id());
        assert!(tmpl.is_unnamed_value());
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
        let tmpl = parse_template_str("%URL {}").unwrap();
        assert_eq!(tmpl.prefix, '%');
        assert_eq!(tmpl.command, "URL");
        assert!(tmpl.value_field.is_some());
        assert!(tmpl.is_unnamed_value());
    }

    #[test]
    fn indexed_with_filename() {
        let tmpl = parse_template_str("#EXBMP{id} {filename}").unwrap();
        assert_eq!(tmpl.prefix, '#');
        assert_eq!(tmpl.command, "EXBMP");
        assert!(tmpl.is_indexed());
        assert_eq!(tmpl.id_field.as_ref().and_then(|p| p.name()), Some("id"));
        assert_eq!(
            tmpl.value_field.as_ref().and_then(|p| p.name()),
            Some("filename")
        );
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
    fn named_value_placeholder_preserved() {
        let tmpl = parse_template_str("#TEXT{id} {value}").unwrap();
        assert!(tmpl.is_indexed());
        assert!(tmpl.id_field.as_ref().unwrap().is_named());
        assert!(tmpl.value_field.as_ref().unwrap().is_named());
        assert_eq!(
            tmpl.value_field.as_ref().and_then(|p| p.name()),
            Some("value")
        );
    }

    #[test]
    fn empty_template_errors() {
        assert!(parse_template_str("").is_err());
    }

    #[test]
    fn missing_prefix_errors() {
        assert!(parse_template_str("TITLE {}").is_err());
    }

    #[test]
    fn unclosed_brace_errors() {
        assert!(parse_template_str("#BPM{id {value}").is_err());
    }

    #[test]
    fn indexed_with_unnamed_id() {
        let tmpl = parse_template_str("#BPM{} {value}").unwrap();
        assert!(tmpl.is_indexed());
        assert!(tmpl.is_unnamed_id());
        assert!(tmpl.value_field.as_ref().unwrap().is_named());
    }
}
