/// CQL syntax tokenizer for highlighting and autocomplete.
///
/// Single-pass O(n) lexer that classifies tokens into keywords, types,
/// functions, strings, numbers, comments, variables, etc. No external
/// parser crates — just sorted slices with binary_search for lookup.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Type,
    Function,
    String,
    Number,
    Comment,
    /// `{{name}}` template variables
    Variable,
    Identifier,
    Operator,
    Punctuation,
    Whitespace,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub start: usize,
}

// ── Sorted lookup tables (binary_search, no deps) ──────────────

static CQL_KEYWORDS: &[&str] = &[
    "ADD", "ALL", "ALLOW", "ALTER", "AND", "ANY", "APPLY", "AS", "ASC",
    "AUTHORIZE", "BATCH", "BEGIN", "BY", "CALLED", "CLUSTERING",
    "COLUMNFAMILY", "COMPACT", "CONTAINS", "CREATE", "CUSTOM",
    "DELETE", "DESC", "DESCRIBE", "DISTINCT", "DROP", "EACH_QUORUM",
    "ENTRIES", "EXECUTE", "EXISTS", "FILTERING", "FINALFUNC", "FROM",
    "FULL", "GRANT", "IF", "IN", "INDEX", "INITCOND", "INPUT", "INSERT",
    "INTO", "IS", "JSON", "KEY", "KEYSPACE", "KEYSPACES", "LANGUAGE",
    "LIMIT", "LOCAL_ONE", "LOCAL_QUORUM", "LOGGED", "LOGIN",
    "MATERIALIZED", "MODIFY", "NORECURSIVE", "NOSUPERUSER", "NOT",
    "NULL", "OF", "ON", "ONE", "OR", "ORDER", "PARTITION", "PASSWORD",
    "PER", "PERMISSION", "PERMISSIONS", "PRIMARY", "QUORUM", "RENAME",
    "REPLACE", "RETURNS", "REVOKE", "SCHEMA", "SELECT", "SET", "SFUNC",
    "STATIC", "STORAGE", "STYPE", "SUPERUSER", "TABLE", "THREE",
    "TO", "TOKEN", "TRIGGER", "TRUNCATE", "TTL", "TWO", "TYPE",
    "UNLOGGED", "UPDATE", "USE", "USER", "USERS", "USING", "VALUES",
    "VIEW", "WHERE", "WITH", "WRITETIME",
];

static CQL_TYPES: &[&str] = &[
    "ASCII", "BIGINT", "BLOB", "BOOLEAN", "COUNTER", "DATE", "DECIMAL",
    "DOUBLE", "DURATION", "FLOAT", "FROZEN", "INET", "INT", "LIST",
    "MAP", "SET", "SMALLINT", "TEXT", "TIME", "TIMESTAMP", "TIMEUUID",
    "TINYINT", "TUPLE", "UUID", "VARCHAR", "VARINT",
];

static CQL_FUNCTIONS: &[&str] = &[
    "AVG", "CAST", "COUNT", "DATEOF", "FROMJSON", "MAX", "MIN",
    "NOW", "SUM", "TOJSON", "TOKEN", "TOUNIXTIMESTAMP", "TTL",
    "UUID", "WRITETIME",
];

fn is_keyword(word: &str) -> bool {
    CQL_KEYWORDS.binary_search(&word).is_ok()
}

fn is_type(word: &str) -> bool {
    CQL_TYPES.binary_search(&word).is_ok()
}

fn is_function(word: &str) -> bool {
    CQL_FUNCTIONS.binary_search(&word).is_ok()
}

// ── Tokenizer ──────────────────────────────────────────────────

pub fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        let start = i;
        let b = bytes[i];

        // 1. Line comment: --
        if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 2. Block comment: /* ... */
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 3. Template variable: {{...}}
        if b == b'{' && i + 1 < len && bytes[i + 1] == b'{' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'}' && bytes[i + 1] == b'}') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip }}
            }
            tokens.push(Token {
                kind: TokenKind::Variable,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 4. String literal: '...' (CQL escapes '' inside)
        if b == b'\'' {
            i += 1;
            loop {
                if i >= len {
                    break;
                }
                if bytes[i] == b'\'' {
                    i += 1;
                    // escaped quote ''
                    if i < len && bytes[i] == b'\'' {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::String,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 5. Quoted identifier: "..."
        if b == b'"' {
            i += 1;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Identifier,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 6. Number (integer or decimal)
        if b.is_ascii_digit() {
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                i += 1;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 7. Word (keyword / type / function / identifier)
        if b.is_ascii_alphabetic() || b == b'_' {
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &source[start..i];
            let upper = word.to_ascii_uppercase();
            let kind = if is_keyword(&upper) {
                TokenKind::Keyword
            } else if is_type(&upper) {
                TokenKind::Type
            } else if is_function(&upper) {
                TokenKind::Function
            } else {
                TokenKind::Identifier
            };
            tokens.push(Token {
                kind,
                text: word.to_string(),
                start,
            });
            continue;
        }

        // 8. Whitespace
        if b.is_ascii_whitespace() {
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 9. Operators
        if matches!(b, b'=' | b'<' | b'>' | b'!' | b'+' | b'-' | b'*' | b'/' | b'.') {
            // Handle two-char operators: !=, <=, >=
            if i + 1 < len && bytes[i + 1] == b'=' && matches!(b, b'!' | b'<' | b'>') {
                i += 2;
            } else {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 10. Punctuation
        if matches!(b, b';' | b',' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b':') {
            i += 1;
            tokens.push(Token {
                kind: TokenKind::Punctuation,
                text: source[start..i].to_string(),
                start,
            });
            continue;
        }

        // 11. Unknown byte — advance one char
        i += 1;
        tokens.push(Token {
            kind: TokenKind::Unknown,
            text: source[start..i].to_string(),
            start,
        });
    }

    tokens
}

// ── HTML output ────────────────────────────────────────────────

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn css_class(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Keyword => Some("cql-keyword"),
        TokenKind::Type => Some("cql-type"),
        TokenKind::Function => Some("cql-function"),
        TokenKind::String => Some("cql-string"),
        TokenKind::Number => Some("cql-number"),
        TokenKind::Comment => Some("cql-comment"),
        TokenKind::Variable => Some("cql-variable"),
        TokenKind::Identifier => Some("cql-identifier"),
        _ => None,
    }
}

pub fn to_highlighted_html(tokens: &[Token]) -> String {
    let mut html = String::with_capacity(tokens.iter().map(|t| t.text.len() + 30).sum());
    for token in tokens {
        let escaped = html_escape(&token.text);
        if let Some(class) = css_class(token.kind) {
            html.push_str("<span class=\"");
            html.push_str(class);
            html.push_str("\">");
            html.push_str(&escaped);
            html.push_str("</span>");
        } else {
            html.push_str(&escaped);
        }
    }
    html
}

// ── Autocomplete helpers ───────────────────────────────────────

/// Extract the partial word being typed at `cursor` position.
/// Returns (partial_word, start_offset).
pub fn word_at_cursor(source: &str, cursor: usize) -> (&str, usize) {
    let bytes = source.as_bytes();
    let end = cursor.min(bytes.len());
    let mut start = end;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    (&source[start..end], start)
}

/// Returns the keyword immediately before `cursor` that is not part of the
/// current word (skipping whitespace). Used to detect context like `FROM <table>`.
pub fn keyword_before_cursor(source: &str, cursor: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let end = cursor.min(bytes.len());

    // Skip back over the current partial word
    let mut i = end;
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        i -= 1;
    }
    // Skip whitespace
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    // Read the previous word
    let word_end = i;
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        i -= 1;
    }
    if i < word_end {
        Some(source[i..word_end].to_ascii_uppercase())
    } else {
        None
    }
}

/// Case-insensitive prefix match against all keywords, types, and functions.
/// Returns at most `limit` suggestions sorted alphabetically.
pub fn suggest_completions(partial: &str, limit: usize) -> Vec<&'static str> {
    if partial.is_empty() {
        return Vec::new();
    }
    let upper = partial.to_ascii_uppercase();
    let mut results: Vec<&'static str> = CQL_KEYWORDS
        .iter()
        .chain(CQL_TYPES.iter())
        .chain(CQL_FUNCTIONS.iter())
        .filter(|w| w.starts_with(&upper))
        .copied()
        .collect();
    results.sort_unstable();
    results.dedup();
    results.truncate(limit);
    results
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_select() {
        let tokens = tokenize("SELECT * FROM ks.table1 WHERE id = 'hello'");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds[0], TokenKind::Keyword); // SELECT
        assert_eq!(kinds[2], TokenKind::Operator); // *
        assert_eq!(kinds[4], TokenKind::Keyword); // FROM
        // 'hello' should be String
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String && t.text == "'hello'"));
    }

    #[test]
    fn tokenize_comment() {
        let tokens = tokenize("-- this is a comment\nSELECT");
        assert_eq!(tokens[0].kind, TokenKind::Comment);
        assert_eq!(tokens[2].kind, TokenKind::Keyword);
    }

    #[test]
    fn tokenize_variable() {
        let tokens = tokenize("SELECT * FROM {{table_name}}");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Variable && t.text == "{{table_name}}"));
    }

    #[test]
    fn tokenize_types() {
        let tokens = tokenize("CREATE TABLE t (id uuid, name text)");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Type && t.text.eq_ignore_ascii_case("uuid")));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Type && t.text.eq_ignore_ascii_case("text")));
    }

    #[test]
    fn word_at_cursor_mid() {
        let (word, start) = word_at_cursor("SELECT * FRO", 12);
        assert_eq!(word, "FRO");
        assert_eq!(start, 9);
    }

    #[test]
    fn suggest_select() {
        let results = suggest_completions("SEL", 10);
        assert!(results.contains(&"SELECT"));
    }

    #[test]
    fn html_escapes() {
        let tokens = tokenize("a < b");
        let html = to_highlighted_html(&tokens);
        assert!(html.contains("&lt;"));
    }

    #[test]
    fn keyword_before_from() {
        let kw = keyword_before_cursor("SELECT * FROM tab", 14);
        assert_eq!(kw.as_deref(), Some("FROM"));
    }
}
