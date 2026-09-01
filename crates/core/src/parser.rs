use crate::ast::{Attr, Element, ElementKind, Node};
use crate::span::Span;
use crate::token::{tokenize, Token};

/// A structural problem found while parsing.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// The result of parsing a single file.
#[derive(Debug)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub errors: Vec<ParseError>,
}

/// Parse a redoc source file into a document tree.
pub fn parse(src: &str) -> Document {
    let toks = tokenize(src);
    let mut p = Parser {
        src,
        toks,
        pos: 0,
        errors: Vec::new(),
    };
    let nodes = p.parse_sequence(Stop::Eof).0;
    Document { nodes, errors: p.errors }
}

#[derive(Clone, Copy, PartialEq)]
enum Stop {
    /// Stop at end of input.
    Eof,
    /// Stop at the next `]` (consuming it).
    Close,
    /// Stop at an end tag whose name is in the list.
    End(&'static [&'static str]),
    /// Like `End`, but end-of-input also closes the region without error.
    /// `[section]` uses this: the end marker is optional.
    EndOrEof(&'static [&'static str]),
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
}

#[derive(Debug, Clone, Default)]
struct TagHead {
    name: String,
    subtype: Option<String>,
    attrs: Vec<Attr>,
    anchor: Option<String>,
    /// For end tags: the name of the region being closed.
    end_for: Option<String>,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    fn err(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(ParseError {
            message: message.into(),
            span,
        });
    }

    /// Parse a sequence of nodes until `stop`.
    ///
    /// Returns the nodes and, when stopped by an end tag, the span of that
    /// end tag.
    fn parse_sequence(&mut self, stop: Stop) -> (Vec<Node>, Option<Span>) {
        let mut nodes = Vec::new();
        let mut end_span = None;
        loop {
            let Some(tok) = self.peek().cloned() else {
                match stop {
                    Stop::Eof | Stop::EndOrEof(_) => break,
                    Stop::Close => {
                        let end = self.src.len();
                        self.err("unclosed tag: missing ']'", Span::new(end, end));
                    }
                    Stop::End(_) => {
                        let end = self.src.len();
                        self.err("unclosed region: missing end marker", Span::new(end, end));
                    }
                }
                break;
            };
            match tok {
                Token::Close { span } => match stop {
                    Stop::Close => {
                        self.pos += 1;
                        end_span = Some(span);
                        break;
                    }
                    _ => {
                        self.err("stray ']'", span);
                        self.pos += 1;
                    }
                },
                Token::Text { text, span } => {
                    self.pos += 1;
                    nodes.push(Node::Text { text, span });
                }
                Token::Comment { text, span } => {
                    self.pos += 1;
                    nodes.push(Node::Comment { text, span });
                }
                Token::Open { span } => {
                    self.pos += 1;
                    let (head, leftover, leftover_span) = self.parse_head();
                    if let Some(en) = head.end_for {
                        // This is an end tag.
                        let matching = match stop {
                            Stop::End(names) | Stop::EndOrEof(names) => names.contains(&en.as_str()),
                            _ => false,
                        };
                        if matching {
                            let (_, _) = self.parse_sequence(Stop::Close);
                            let end = end_span.unwrap_or(span);
                            end_span = Some(Span::new(span.start, end.end));
                            break;
                        } else {
                            let msg = format!(
                                "stray or mismatched end tag `[{}{}]`",
                                if head.name.is_empty() {
                                    String::new()
                                } else {
                                    head.name.clone()
                                },
                                if head.name.is_empty() {
                                    format!("end:{}", en)
                                } else {
                                    format!(":end")
                                }
                            );
                            self.err(msg, span);
                            let (_, _) = self.parse_sequence(Stop::Close);
                        }
                    } else {
                        // Normal tag: content inside the brackets.
                        let mut content = Vec::new();
                        if !leftover.is_empty() {
                            content.push(Node::Text {
                                text: leftover,
                                span: leftover_span,
                            });
                        }
                        let (inner, _) = self.parse_sequence(Stop::Close);
                        content.extend(inner);

                        if is_sublang(&head.name) {
                            // Sub-language region: capture body raw.
                            let marker = format!("[{}:end]", head.name);
                            let (raw, es) = self.capture_raw_until(&marker);
                            nodes.push(Node::Element(Element {
                                name: head.name,
                                subtype: head.subtype,
                                attrs: head.attrs,
                                anchor: head.anchor,
                                content,
                                body: Vec::new(),
                                raw: Some(raw),
                                kind: ElementKind::SubLanguage,
                                span,
                                end_span: es,
                            }));
                        } else if head.name == "section" {
                            // [section] end marker is optional: close at
                            // [section:end] or at end of input.
                            let (body, es) = self.parse_sequence(Stop::EndOrEof(&["section"]));
                            nodes.push(Node::Element(Element {
                                name: head.name,
                                subtype: head.subtype,
                                attrs: head.attrs,
                                anchor: head.anchor,
                                content,
                                body,
                                raw: None,
                                kind: ElementKind::Region,
                                span,
                                end_span: es,
                            }));
                        } else if head.name == "document" {
                            // The document region has no end marker; body runs
                            // to EOF.
                            let (body, _) = self.parse_sequence(Stop::Eof);
                            nodes.push(Node::Element(Element {
                                name: head.name,
                                subtype: head.subtype,
                                attrs: head.attrs,
                                anchor: head.anchor,
                                content,
                                body,
                                raw: None,
                                kind: ElementKind::Region,
                                span,
                                end_span: None,
                            }));
                        } else if let Some(end_names) = end_names(&head.name, head.subtype.as_deref()) {
                            let (body, es) = self.parse_sequence(Stop::End(end_names));
                            nodes.push(Node::Element(Element {
                                name: head.name,
                                subtype: head.subtype,
                                attrs: head.attrs,
                                anchor: head.anchor,
                                content,
                                body,
                                raw: None,
                                kind: ElementKind::Region,
                                span,
                                end_span: es,
                            }));
                        } else {
                            nodes.push(Node::Element(Element {
                                name: head.name,
                                subtype: head.subtype,
                                attrs: head.attrs,
                                anchor: head.anchor,
                                content,
                                body: Vec::new(),
                                raw: None,
                                kind: ElementKind::Leaf,
                                span,
                                end_span: None,
                            }));
                        }
                    }
                }
            }
        }
        (nodes, end_span)
    }

    /// Parse the head of a tag: everything after `[` up to the first `[` or
    /// `]`, i.e. name/subtype/attrs/anchor plus any leftover text that
    /// begins the content.
    fn parse_head(&mut self) -> (TagHead, String, Span) {
        let mut head_text = String::new();
        let tok_span = match self.peek() {
            Some(Token::Text { span, .. }) => *span,
            _ => Span::new(self.src.len(), self.src.len()),
        };
        if let Some(Token::Text { text, .. }) = self.peek() {
            head_text = text.clone();
            self.pos += 1;
        }
        let (head, head_len) = parse_head_str(&head_text);
        let leftover = head_text[head_len..].to_string();
        let leftover_span = Span::new(tok_span.start + head_len, tok_span.end);
        (head, leftover, leftover_span)
    }

    /// Capture raw text from the current position up to `marker`, skipping
    /// the marker, and advance the token stream past it.
    fn capture_raw_until(&mut self, marker: &str) -> (String, Option<Span>) {
        let start = match self.peek() {
            Some(t) => t.span().start,
            None => self.src.len(),
        };
        let Some(rel) = self.src[start..].find(marker) else {
            let end = self.src.len();
            self.err(format!("missing closing `{marker}`"), Span::new(end, end));
            self.pos = self.toks.len();
            return (self.src[start..].to_string(), None);
        };
        let marker_start = start + rel;
        let marker_end = marker_start + marker.len();
        let raw = self.src[start..marker_start].to_string();
        while self.pos < self.toks.len() && self.toks[self.pos].span().end <= marker_end {
            self.pos += 1;
        }
        (raw, Some(Span::new(marker_start, marker_end)))
    }
}

/// Regions whose body is a sub-language and is captured raw.
fn is_sublang(name: &str) -> bool {
    matches!(name, "codeblock" | "math" | "formula" | "figure")
}

/// The set of end-tag names that close a given region.
fn end_names(name: &str, subtype: Option<&str>) -> Option<&'static [&'static str]> {
    match name {
        "list" => Some(&["list"]),
        "div" => Some(&["div"]),
        "syntax" => Some(&["syntax"]),
        "table" => Some(&["table"]),
        "rule" => Some(&["rule"]),
        "note" => Some(&["note"]),
        "begin" => match subtype {
            Some("note") => Some(&["note"]),
            Some("example") => Some(&["example"]),
            _ => None,
        },
        _ => None,
    }
}

/// Split a word: leading `[A-Za-z0-9_]+`.
fn split_word(s: &str) -> (&str, &str) {
    let n = s
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    (&s[..n], &s[n..])
}

/// Split letters for subtypes: `[A-Za-z0-9_~]+`.
fn split_letters(s: &str) -> (&str, &str) {
    let n = s
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'~')
        .count();
    (&s[..n], &s[n..])
}

/// Parse `@name` / `@name=value` attributes and an optional `#anchor`.
/// Returns (attrs, anchor, bytes-consumed).
fn parse_attrs_anchor(rest: &str) -> (Vec<Attr>, Option<String>, usize) {
    let mut attrs = Vec::new();
    let mut anchor = None;
    let mut r = rest;
    loop {
        if let Some(rem) = r.strip_prefix('@') {
            let (name, r2) = split_word(rem);
            if name.is_empty() {
                break;
            }
            let (value, r3) = if let Some(rv) = r2.strip_prefix('=') {
                let end = rv
                    .find(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '@' | '#'))
                    .unwrap_or(rv.len());
                (Some(rv[..end].to_string()), &rv[end..])
            } else {
                (None, r2)
            };
            attrs.push(Attr {
                name: name.to_string(),
                value,
            });
            r = r3;
        } else if let Some(rem) = r.strip_prefix('#') {
            let end = rem.find(char::is_whitespace).unwrap_or(rem.len());
            anchor = Some(rem[..end].to_string());
            r = &rem[end..];
            break;
        } else {
            break;
        }
    }
    (attrs, anchor, rest.len() - r.len())
}

/// Parse the head text of a tag, returning the head and the number of bytes
/// consumed.
fn parse_head_str(s: &str) -> (TagHead, usize) {
    let mut head = TagHead::default();
    let mut consumed = 0usize;
    let Some(c) = s.chars().next() else {
        return (head, 0);
    };
    match c {
        // Evaluation tag: `[?name @attrs #anchor content]`
        '?' => {
            consumed += c.len_utf8();
            let rest = &s[consumed..];
            let (w, r) = split_word(rest);
            head.name = w.to_string();
            consumed += rest.len() - r.len();
            let (attrs, anchor, adv) = parse_attrs_anchor(r);
            head.attrs = attrs;
            head.anchor = anchor;
            consumed += adv;
        }
        // Inline code: `` [`(:mod){0,2}(@def|@lib)? content] ``
        '`' => {
            head.name = "`".to_string();
            consumed += 1;
            let mut rest = &s[1..];
            for _ in 0..2 {
                if let Some(r) = rest.strip_prefix(':') {
                    let (w, r2) = split_word(r);
                    if matches!(w, "opt" | "key" | "c" | "cname" | "m") {
                        head.subtype = Some(w.to_string());
                        consumed += 1 + (r.len() - r2.len());
                        rest = r2;
                        continue;
                    }
                }
                break;
            }
            let (attrs, anchor, adv) = parse_attrs_anchor(rest);
            head.attrs = attrs;
            head.anchor = anchor;
            consumed += adv;
        }
        // Marker tags: `[~...]` `[*...]` `[^...]` `[$...]` `[=...]` `[#...]`
        // `[+...]` `[%...]` `[!...]` `[&...]` `["...]` `[|...]` `[-]`
        '~' | '*' | '^' | '$' | '=' | '#' | '+' | '%' | '!' | '&' | '"' | '|' | '-' => {
            head.name = c.to_string();
            consumed += c.len_utf8();
            let mut rest = &s[consumed..];
            if let Some(r) = rest.strip_prefix(':') {
                let (w, r2) = split_letters(r);
                head.subtype = Some(w.to_string());
                consumed += 1 + (r.len() - r2.len());
                rest = r2;
            }
            let (attrs, anchor, adv) = parse_attrs_anchor(rest);
            head.attrs = attrs;
            head.anchor = anchor;
            consumed += adv;
        }
        // Language tag: `[:en]` `[:zh_CN]` `[:]`
        ':' => {
            consumed += 1;
            let rest = &s[1..];
            let (w, r) = split_letters(rest);
            head.subtype = Some(w.to_string());
            consumed += rest.len() - r.len();
            let (attrs, _, adv) = parse_attrs_anchor(r);
            head.attrs = attrs;
            consumed += adv;
        }
        // Word-named tag: `[name:subtype @attrs #anchor content]`
        _ if c.is_ascii_alphabetic() || c == '_' => {
            let (w, r) = split_word(s);
            head.name = w.to_string();
            consumed += s.len() - r.len();
            let mut rest = r;
            if let Some(r) = rest.strip_prefix(':') {
                let (w2, r2) = split_letters(r);
                head.subtype = Some(w2.to_string());
                consumed += 1 + (r.len() - r2.len());
                rest = r2;
            }
            let (attrs, anchor, adv) = parse_attrs_anchor(rest);
            head.attrs = attrs;
            head.anchor = anchor;
            consumed += adv;
            head.end_for = match head.name.as_str() {
                "end" if head.subtype.is_some() => head.subtype.clone(),
                _ if head.subtype.as_deref() == Some("end") => Some(head.name.clone()),
                _ => None,
            };
        }
        _ => {
            // Unknown head: treat the whole token as content.
            head.name = String::new();
            return (head, 0);
        }
    }
    (head, consumed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Vec<Node> {
        let doc = parse(src);
        assert!(doc.errors.is_empty(), "errors: {:?}", doc.errors);
        doc.nodes
    }

    fn elem(n: &Node) -> &Element {
        match n {
            Node::Element(e) => e,
            other => panic!("expected element, got {:?}", other),
        }
    }

    /// Nodes, dropping whitespace-only text nodes.
    fn sig(nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .filter(|n| match n {
                Node::Text { text, .. } => !text.trim().is_empty(),
                _ => true,
            })
            .cloned()
            .collect()
    }

    #[test]
    fn parses_para() {
        let nodes = sig(&parse_ok("[para]\n[:en] Hello\n[:zh_CN] 你好\n"));
        assert_eq!(nodes.len(), 5);
        assert_eq!(elem(&nodes[0]).name, "para");
        let lang = elem(&nodes[1]);
        assert_eq!(lang.name, "");
        assert_eq!(lang.subtype.as_deref(), Some("en"));
        match &nodes[2] {
            Node::Text { text, .. } => assert_eq!(text.trim(), "Hello"),
            other => panic!("expected text, got {:?}", other),
        }
    }

    #[test]
    fn parses_section_region() {
        let nodes = parse_ok("[section:chapter#lex\n[:en] Lexical conventions\n]\n[para]x\n[section:end]\n");
        assert_eq!(sig(&nodes).len(), 1);
        let s = elem(&nodes[0]);
        assert_eq!(s.name, "section");
        assert_eq!(s.subtype.as_deref(), Some("chapter"));
        assert_eq!(s.anchor.as_deref(), Some("lex"));
        assert_eq!(s.kind, ElementKind::Region);
        let body = sig(&s.body);
        assert_eq!(body.len(), 2); // [para] + trailing text "x"
        assert_eq!(elem(&body[0]).name, "para");
        assert_eq!(s.end_span.is_some(), true);
    }

    #[test]
    fn parses_codeblock_raw() {
        let nodes = parse_ok("[codeblock]\nint x; // [:en] comment\n[codeblock:end]\n");
        let cb = elem(&nodes[0]);
        assert_eq!(cb.kind, ElementKind::SubLanguage);
        assert_eq!(cb.raw.as_deref(), Some("\nint x; // [:en] comment\n"));
    }

    #[test]
    fn parses_marker_tags() {
        let nodes = parse_ok("[~identifier] [=Cpp] [#lex.token] [%type[!sub]] [^:oc CopyConstructible]");
        assert_eq!(elem(&nodes[0]).name, "~");
        assert_eq!(elem(&nodes[2]).name, "=");
        assert_eq!(elem(&nodes[4]).name, "#");
        assert_eq!(elem(&nodes[6]).name, "%");
        assert_eq!(elem(&nodes[8]).name, "^");
        assert_eq!(elem(&nodes[8]).subtype.as_deref(), Some("oc"));
    }

    #[test]
    fn parses_table_cells() {
        // Header cells live inside the table's opening brackets; rows and
        // separators are body until [table:end].
        let nodes = parse_ok("[table:grid#t\n[|@headerspan=2 x]\n]\n[-]\n[|]\n[table:end]\n");
        let t = elem(&nodes[0]);
        assert_eq!(t.name, "table");
        let content = sig(&t.content);
        assert_eq!(content.len(), 1);
        let cell = elem(&content[0]);
        assert_eq!(cell.name, "|");
        assert_eq!(cell.attrs[0].name, "headerspan");
        assert_eq!(cell.attrs[0].value.as_deref(), Some("2"));
        let body = sig(&t.body);
        assert_eq!(body.len(), 2);
        assert_eq!(elem(&body[0]).name, "-");
        assert_eq!(elem(&body[1]).name, "|");
    }

    #[test]
    fn reports_unclosed_tag() {
        let doc = parse("[para\n");
        assert_eq!(doc.errors.len(), 1);
        assert!(doc.errors[0].message.contains("unclosed"));
    }

    #[test]
    fn reports_stray_close() {
        let doc = parse("text ] more");
        assert_eq!(doc.errors.len(), 1);
        assert!(doc.errors[0].message.contains("stray"));
    }

    #[test]
    fn parses_eval_tag() {
        let nodes = parse_ok("[?impldef forward progress guarantees\n]");
        let e = elem(&nodes[0]);
        assert_eq!(e.name, "impldef");
    }

    #[test]
    fn parses_end_note() {
        let nodes = parse_ok("[begin:note]\n[para]\n[end:note]\n");
        let n = elem(&nodes[0]);
        assert_eq!(n.name, "begin");
        assert_eq!(n.kind, ElementKind::Region);
        let body = sig(&n.body);
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn inline_code_nested() {
        // From basic.fundamental: ``[`[`:key signed] [`:key char]]`` renders
        // as inline code containing two nested keyword elements.
        let nodes = parse_ok("[`[`:key signed] [`:key char]]");
        let outer = elem(&nodes[0]);
        assert_eq!(outer.name, "`");
        let inner = sig(&outer.content);
        assert_eq!(inner.len(), 2);
        assert_eq!(elem(&inner[0]).name, "`");
        assert_eq!(elem(&inner[0]).subtype.as_deref(), Some("key"));
        assert_eq!(elem(&inner[1]).subtype.as_deref(), Some("key"));
    }

    #[test]
    fn inline_code_escaped_brackets() {
        // `[`[`]] renders the literal `[` `]` operators inside inline code:
        // [ + backtick name + escape `[ + escape `] + ].
        let nodes = parse_ok("[``[`]]");
        let e = elem(&nodes[0]);
        assert_eq!(e.name, "`");
        let t = match &e.content[0] {
            Node::Text { text, .. } => text.clone(),
            other => panic!("expected text, got {:?}", other),
        };
        assert_eq!(t, "[]");
    }
}
