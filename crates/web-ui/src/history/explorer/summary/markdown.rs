//! Deterministic plain-text normalization for Activity summaries.

// --- Markdown plain-text toolchain -----------------------------------------

/// Escapable punctuation in Markdown escapes (``[\\`*_[\]{}()#+\-.!~>]``).
fn is_escapable(c: char) -> bool {
    matches!(
        c,
        '\\' | '`'
            | '*'
            | '_'
            | '['
            | ']'
            | '{'
            | '}'
            | '('
            | ')'
            | '#'
            | '+'
            | '-'
            | '.'
            | '!'
            | '~'
            | '>'
    )
}

/// Single-emphasis boundary set used by `markdownPlainInline` (JS
/// `[\s([{<:;,.!?-]` and `[\s)\]}>:;,.!?-]`).
fn is_boundary_before(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '(' | '[' | '{' | '<' | ':' | ';' | ',' | '.' | '!' | '?' | '-'
        )
}

fn is_boundary_after(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ')' | ']' | '}' | '>' | ':' | ';' | ',' | '.' | '!' | '?' | '-'
        )
}

/// Bounds-checked char read (the strip scanners never index out of range).
fn char_at(chars: &[char], index: usize) -> char {
    chars.get(index).copied().unwrap_or('\0')
}

/// `markdownPlainInline` — plain text of one Markdown fragment (12-step port).
pub(super) fn markdown_plain_inline(value: &str) -> String {
    let mut chars: Vec<char> = value.chars().collect();
    // \\([\\`*_[\]{}()#+\-.!~>]) -> '$1'
    chars = unescape_punctuation(&chars);
    // !\[([^\]\n]*)\]\([^\n)]*\) -> label
    chars = strip_bracket_links(&chars, true);
    // \[([^\]\n]+)\]\([^\n)]*\) -> label
    chars = strip_bracket_links(&chars, false);
    // `+([^`\n]*?)`+ -> '$1'
    chars = strip_code_spans(&chars);
    // \*\*([^*\n]+)\*\* / __([^_\n]+)__ / ~~([^~\n]+)~~ -> '$1'
    chars = strip_paired('*', &chars);
    chars = strip_paired('_', &chars);
    chars = strip_paired('~', &chars);
    // (^|[\s([{<:;,.!?-])\*([^*\n]+)\*(?=$|[\s)\]}>:;,.!?-]) -> '$1$2'
    chars = strip_single_emphasis('*', &chars);
    chars = strip_single_emphasis('_', &chars);
    // <\/?[A-Za-z][^>\n]*> -> ''
    chars = strip_html_tags(&chars);
    // \*{2,}|~{2,}|`+ -> ''
    chars = strip_delimiter_runs(&chars);
    // \s+ -> ' ', then trim.
    chars
        .into_iter()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn unescape_punctuation(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        if char_at(chars, index) == '\\'
            && let Some(next) = chars.get(index.saturating_add(1))
            && is_escapable(*next)
        {
            out.push(*next);
            index = index.saturating_add(2);
            continue;
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// `!\[([^\]\n]*)\]\([^\n)]*\)` (image, empty label allowed) and
/// `\[([^\]\n]+)\]\([^\n)]*\)` (text link, non-empty label) -> the label.
fn strip_bracket_links(chars: &[char], image: bool) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        let mut cursor = index;
        let is_image = image && chars.get(cursor) == Some(&'!');
        if is_image {
            cursor = cursor.saturating_add(1);
        }
        if chars.get(cursor) == Some(&'[') {
            let label_start = cursor.saturating_add(1);
            let mut label_end = None;
            for (offset, c) in chars.iter().enumerate().skip(label_start) {
                if *c == ']' {
                    label_end = Some(offset);
                    break;
                }
                if *c == '\n' {
                    break;
                }
            }
            if let Some(label_end) = label_end {
                let label_len = label_end.saturating_sub(label_start);
                if image || label_len > 0 {
                    let paren = label_end.saturating_add(1);
                    if chars.get(paren) == Some(&'(') {
                        let target_start = paren.saturating_add(1);
                        let mut target_end = None;
                        for (offset, c) in chars.iter().enumerate().skip(target_start) {
                            if *c == ')' || *c == '\n' {
                                target_end = Some(offset);
                                break;
                            }
                        }
                        if let Some(target_end) = target_end
                            && chars.get(target_end) == Some(&')')
                        {
                            for c in chars.iter().skip(label_start).take(label_len) {
                                out.push(*c);
                            }
                            index = target_end.saturating_add(1);
                            continue;
                        }
                    }
                }
            }
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// `` `+([^`\n]*?)`+ `` -> the inner text (opening/closing runs independent).
fn strip_code_spans(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        if char_at(chars, index) == '`' {
            let open_run = chars.iter().skip(index).take_while(|c| **c == '`').count();
            let content_start = index.saturating_add(open_run);
            let mut close_start = None;
            for (offset, c) in chars.iter().enumerate().skip(content_start) {
                if *c == '\n' {
                    break;
                }
                if *c == '`' {
                    close_start = Some(offset);
                    break;
                }
            }
            if let Some(close_start) = close_start {
                let close_run = chars
                    .iter()
                    .skip(close_start)
                    .take_while(|c| **c == '`')
                    .count();
                for c in chars
                    .iter()
                    .skip(content_start)
                    .take(close_start.saturating_sub(content_start))
                {
                    out.push(*c);
                }
                index = close_start.saturating_add(close_run);
                continue;
            }
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// `**([^*\n]+)**` / `__([^_\n]+)__` / `~~([^~\n]+)~~` -> the inner text.
fn strip_paired(delimiter: char, chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        let pair_here = chars.get(index) == Some(&delimiter)
            && chars.get(index.saturating_add(1)) == Some(&delimiter);
        if !pair_here {
            out.push(char_at(chars, index));
            index = index.saturating_add(1);
            continue;
        }
        let mut inner_start = index.saturating_add(2);
        let mut inner_end = None;
        while inner_start < chars.len() {
            let c = char_at(chars, inner_start);
            if c == '\n' || c == delimiter {
                if c == delimiter && chars.get(inner_start.saturating_add(1)) == Some(&delimiter) {
                    inner_end = Some(inner_start);
                }
                break;
            }
            inner_start = inner_start.saturating_add(1);
        }
        if let Some(inner_end) = inner_end {
            let content_len = inner_end.saturating_sub(index.saturating_add(2));
            if content_len > 0 {
                for c in chars.iter().skip(index.saturating_add(2)).take(content_len) {
                    out.push(*c);
                }
                index = inner_end.saturating_add(2);
                continue;
            }
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// Single-emphasis strip with boundary lookarounds; the boundary char before
/// the opening mark was already emitted, so only the marks are dropped.
fn strip_single_emphasis(delimiter: char, chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        if char_at(chars, index) == delimiter {
            let boundary_before = index == 0
                || chars
                    .get(index.saturating_sub(1))
                    .is_some_and(|c| is_boundary_before(*c));
            let mut inner_start = index.saturating_add(1);
            let mut inner_end = None;
            while inner_start < chars.len() {
                let c = char_at(chars, inner_start);
                if c == '\n' || c == delimiter {
                    if c == delimiter {
                        let after = chars.get(inner_start.saturating_add(1)).copied();
                        if after.is_none_or(is_boundary_after) {
                            inner_end = Some(inner_start);
                        }
                    }
                    break;
                }
                inner_start = inner_start.saturating_add(1);
            }
            if boundary_before {
                let next_is_text = chars
                    .get(index.saturating_add(1))
                    .is_some_and(|c| !c.is_whitespace());
                if let Some(inner_end) = inner_end {
                    let content_len = inner_end.saturating_sub(index.saturating_add(1));
                    if next_is_text && content_len > 0 {
                        for c in chars.iter().skip(index.saturating_add(1)).take(content_len) {
                            out.push(*c);
                        }
                        index = inner_end.saturating_add(1);
                        continue;
                    }
                }
            }
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// Remove each tag through its first closing angle bracket, retaining inner text.
/// The renderer never interprets recorded text as HTML.
fn strip_html_tags(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        if char_at(chars, index) == '<' {
            let mut cursor = index.saturating_add(1);
            if chars.get(cursor) == Some(&'/') {
                cursor = cursor.saturating_add(1);
            }
            let letter = chars.get(cursor).copied();
            if letter.is_some_and(|c| c.is_ascii_alphabetic()) {
                cursor = cursor.saturating_add(1);
                let mut gt = None;
                for (offset, c) in chars.iter().enumerate().skip(cursor) {
                    if *c == '>' {
                        gt = Some(offset);
                        break;
                    }
                    if *c == '\n' {
                        break;
                    }
                }
                if let Some(gt) = gt {
                    index = gt.saturating_add(1);
                    continue;
                }
            }
        }
        out.push(char_at(chars, index));
        index = index.saturating_add(1);
    }
    out
}

/// `\*{2,}|~{2,}|`+ -> '' (unmatched decoration runs are removed).
fn strip_delimiter_runs(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        let c = char_at(chars, index);
        let run = chars.iter().skip(index).take_while(|x| **x == c).count();
        let remove_run = run >= 2 && (c == '*' || c == '~');
        let remove_run = remove_run || c == '`';
        if remove_run {
            index = index.saturating_add(run);
            continue;
        }
        out.push(c);
        index = index.saturating_add(1);
    }
    out
}

/// `markdownPlainLine` — plain-text form of one Markdown source line. Block
/// prefixes lose punctuation while retaining the actual sentence.
pub(super) fn markdown_plain_line(value: &str) -> String {
    let mut line = value.trim().to_owned();
    line = strip_heading_prefix(&line);
    line = strip_task_marker_prefix(&line);
    line = strip_list_marker_prefix(&line);
    line = strip_ordered_marker_prefix(&line);
    line = strip_quote_marker_prefix(&line);
    line = strip_callout_marker_prefix(&line);
    line = strip_fence_marker_prefix(&line);
    markdown_plain_inline(&line)
}

/// Strip one leading prefix with its following whitespace run (JS `\s+`).
fn strip_heading_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let hashes = chars.iter().take_while(|c| **c == '#').count();
    if !(1..=6).contains(&hashes) || !chars.get(hashes).is_some_and(|c| c.is_whitespace()) {
        return line.to_owned();
    }
    let mut cursor = hashes;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}

/// `^[-+*]\s+\[[ xX]\]\s+` strip.
fn strip_task_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if !matches!(chars.first(), Some('-' | '+' | '*')) {
        return line.to_owned();
    }
    let mut cursor = 1;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    if chars.get(cursor) != Some(&'[') {
        return line.to_owned();
    }
    let state = chars.get(cursor.saturating_add(1)).copied();
    if !matches!(state, Some(' ' | 'x' | 'X')) || chars.get(cursor.saturating_add(2)) != Some(&']')
    {
        return line.to_owned();
    }
    let mut after = cursor.saturating_add(3);
    if !chars.get(after).is_some_and(|c| c.is_whitespace()) {
        return line.to_owned();
    }
    while chars.get(after).is_some_and(|c| c.is_whitespace()) {
        after = after.saturating_add(1);
    }
    chars.iter().skip(after).copied().collect()
}

/// `^[-+*]\s+` strip.
fn strip_list_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if !matches!(chars.first(), Some('-' | '+' | '*')) {
        return line.to_owned();
    }
    let mut cursor = 1;
    if !chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        return line.to_owned();
    }
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}

/// `^\d+[.)]\s+` strip.
fn strip_ordered_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
    let sep = chars.get(digits).copied();
    if digits == 0 || !matches!(sep, Some('.' | ')')) {
        return line.to_owned();
    }
    let mut cursor = digits.saturating_add(1);
    if !chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        return line.to_owned();
    }
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}

/// `^>\s*` strip.
fn strip_quote_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.first() != Some(&'>') {
        return line.to_owned();
    }
    let mut cursor = 1;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}

/// `^\[![A-Za-z]+\]\s*` strip.
fn strip_callout_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.first() != Some(&'[') || chars.get(1) != Some(&'!') {
        return line.to_owned();
    }
    let letters = chars
        .iter()
        .skip(2)
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    if letters == 0 || chars.get(2usize.saturating_add(letters)) != Some(&']') {
        return line.to_owned();
    }
    let mut cursor = 2usize.saturating_add(letters).saturating_add(1);
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}

/// ``^`{3,}\s*[A-Za-z0-9_+.-]*\s*`` strip.
fn strip_fence_marker_prefix(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let ticks = chars.iter().take_while(|c| **c == '`').count();
    if ticks < 3 {
        return line.to_owned();
    }
    let mut cursor = ticks;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    while chars
        .get(cursor)
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '.'))
    {
        cursor = cursor.saturating_add(1);
    }
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor = cursor.saturating_add(1);
    }
    chars.iter().skip(cursor).copied().collect()
}
