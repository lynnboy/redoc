use crate::span::Span;

/// A raw token produced by the scanner.
///
/// The scanner splits the source at every `[` and `]` (and `[/` ... `/]`
/// comments), resolving backtick escapes. Structural interpretation is left
/// to the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A run of plain text (escapes already resolved).
    Text { text: String, span: Span },
    /// A literal `[`.
    Open { span: Span },
    /// A literal `]`.
    Close { span: Span },
    /// A comment `[/ ... /]`.
    Comment { text: String, span: Span },
}

impl Token {
    pub fn span(&self) -> Span {
        match self {
            Token::Text { span, .. }
            | Token::Open { span }
            | Token::Close { span }
            | Token::Comment { span, .. } => *span,
        }
    }
}

/// Scan `src` into a token stream.
///
/// Escapes: `` `[ ``, `` `] ``, `` `` ` ``, `` `, `` produce the literal
/// character. Everything else is passed through unchanged.
pub fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut toks = Vec::new();
    let mut buf = String::new();
    let mut buf_start = 0usize;

    let flush = |toks: &mut Vec<Token>, buf: &mut String, buf_start: &mut usize, end: usize| {
        if !buf.is_empty() {
            toks.push(Token::Text {
                text: std::mem::take(buf),
                span: Span::new(*buf_start, end),
            });
        }
        *buf_start = end;
    };

    let mut i = 0usize;
    // When a backtick immediately follows an `[` that opened a tag (i.e. the
    // inline-code marker `` [` ``), it is the tag name, NOT an escape such as
    // `` `[ ``. Track the last-emitted token to disambiguate.
    let mut prev_open = false;
    while i < chars.len() {
        let (pos, c) = chars[i];
        match c {
            '`' => {
                if !prev_open && i + 1 < chars.len() {
                    let n = chars[i + 1].1;
                    if matches!(n, '[' | ']' | '`' | ',') {
                        if buf.is_empty() {
                            buf_start = pos;
                        }
                        buf.push(n);
                        i += 2;
                        prev_open = false;
                        continue;
                    }
                }
                if buf.is_empty() {
                    buf_start = pos;
                }
                buf.push('`');
                i += 1;
                prev_open = false;
            }
            '[' => {
                if i + 1 < chars.len() && chars[i + 1].1 == '/' {
                    // comment: [/ ... /]
                    flush(&mut toks, &mut buf, &mut buf_start, pos);
                    let mut j = i + 2;
                    let mut end = src.len();
                    while j + 1 < chars.len() {
                        if chars[j].1 == '/' && chars[j + 1].1 == ']' {
                            end = chars[j + 1].0 + 1;
                            break;
                        }
                        j += 1;
                    }
                    let text = src[pos + 2..end.saturating_sub(2)].to_string();
                    toks.push(Token::Comment {
                        text,
                        span: Span::new(pos, end),
                    });
                    i = if end >= src.len() { chars.len() } else { j + 2 };
                    buf_start = end;
                    prev_open = false;
                } else {
                    flush(&mut toks, &mut buf, &mut buf_start, pos);
                    toks.push(Token::Open {
                        span: Span::new(pos, pos + 1),
                    });
                    buf_start = pos + 1;
                    i += 1;
                    prev_open = true;
                }
            }
            ']' => {
                flush(&mut toks, &mut buf, &mut buf_start, pos);
                toks.push(Token::Close {
                    span: Span::new(pos, pos + 1),
                });
                buf_start = pos + 1;
                i += 1;
                prev_open = false;
            }
            _ => {
                if buf.is_empty() {
                    buf_start = pos;
                }
                buf.push(c);
                i += 1;
                prev_open = false;
            }
        }
    }
    flush(&mut toks, &mut buf, &mut buf_start, src.len());
    toks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_brackets() {
        let toks = tokenize("a [para] b");
        assert_eq!(toks.len(), 5);
        assert_eq!(toks[0], Token::Text { text: "a ".into(), span: Span::new(0, 2) });
        assert_eq!(toks[1], Token::Open { span: Span::new(2, 3) });
        assert_eq!(toks[2], Token::Text { text: "para".into(), span: Span::new(3, 7) });
        assert_eq!(toks[3], Token::Close { span: Span::new(7, 8) });
        assert_eq!(toks[4], Token::Text { text: " b".into(), span: Span::new(8, 10) });
    }

    #[test]
    fn resolves_escapes() {
        let toks = tokenize("`[`]``");
        // `` `[ `` -> `[`, `` `] `` -> `]`, ``` `` ``` -> `` ` ``
        let texts: Vec<String> = toks
            .into_iter()
            .filter_map(|t| match t {
                Token::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["[]`"]);
    }

    #[test]
    fn comment_token() {
        let toks = tokenize("[/ hello /]");
        assert_eq!(toks.len(), 1);
        assert!(matches!(&toks[0], Token::Comment { text, .. } if text == " hello "));
    }
}
