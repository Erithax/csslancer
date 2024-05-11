
use super::cursor::Cursor;

pub fn unescape(content: &str) -> String {
    let mut cur = Cursor::new(content);
    let mut res = String::new();
    if unquoted_string(&mut cur, &mut res) {
        return res
    }
    content.to_string()
}



fn unquoted_string(cur: &mut Cursor, result: &mut String) -> bool {
    let mut has_content = false;
    while unquoted_char(cur, result) || escape(cur, result, false) {
        has_content = true;
    }
    has_content
}

fn unquoted_char(cur: &mut Cursor, result: &mut String) -> bool {
    // not closeQuote, not backslash, not whitespace, not newline
    let ch = cur.first();
    match ch  {
        '\0' | '\\' | '\'' | '"' | '(' | ')' | ' ' | '\t' | '\n' | '\x0c' | '\r' => false,
        _ => {
            cur.bump();
            result.push(ch);
            true
        }
    }
}

fn escape(cur: &mut Cursor, result: &mut String, include_new_lines: bool) -> bool {
    let mut ch = cur.first();
    if ch == '\\' {
        cur.bump();
        ch = cur.first();
        let mut hex_str = String::new();
        while hex_str.len() < 6 && ch.is_ascii_hexdigit() {
            hex_str.push(ch);
            cur.bump();
            ch = cur.first();
        }
        if !hex_str.is_empty() {
            let c = char::from_u32(u32::from_str_radix(&hex_str, 16).unwrap()).unwrap();
            result.push(c);

            // optional whitespace or new line, not part of result text
            if ch == ' ' || ch == '\t' {
                cur.bump();
            } else {
                newline(cur, result);
            }
            return true;
        }
        if ch != '\r' && ch != '\x0c' && ch != '\n' {
            cur.bump();
            result.push(ch);
            return true;
        } else if include_new_lines {
            return newline(cur, result);
        }
    }
    false
}

fn newline(cur: &mut Cursor, result: &mut String) -> bool {
    let ch = cur.first();
    if ch == '\r' || ch == '\x0c' || ch == '\n' {
        cur.bump();
        result.push(ch);
        if ch == '\r' && cur.first() == '\n' {
            cur.bump();
            result.push('\n');
        }
        return true
    }
    false
}



#[cfg(test)]
mod extra_test {

    use super::unescape;

    #[test]
    fn test_unescape() {
        println!("\u{f60e}");
        assert_eq!(unescape(r#"\34"#), "4");
        assert_eq!(unescape(r#"\1f60e"#), "😎");
        assert_eq!(unescape(r#"\1F916 "#), "🤖")
    }
}