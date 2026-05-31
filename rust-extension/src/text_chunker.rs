fn is_chinese_char(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c)
        || ('\u{3400}'..='\u{4dbf}').contains(&c)
        || ('\u{f900}'..='\u{faff}').contains(&c)
}

fn is_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | '.' | '!' | '?' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | '，' | '。' | '！' | '？' | '；' | '：' | '「' | '」' | '『' | '』' | '（' | '）' | '【' | '】' | '《' | '》' | '…' | '—' | '～'
    )
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || c == '\n' || c == '\r' || is_punctuation(c)
}

fn estimate_tokens(word: &str) -> f64 {
    if word.is_empty() {
        return 0.0;
    }

    let first_char = word.chars().next().unwrap();

    if is_chinese_char(first_char) {
        let char_count = word.chars().filter(|c| is_chinese_char(*c)).count();
        if char_count == 0 {
            return word.len() as f64 * 0.3;
        }
        return char_count as f64 / 1.5;
    }

    word.len() as f64 * 0.3
}

fn split_into_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut current_is_chinese = false;

    for c in text.chars() {
        let c_is_chinese = is_chinese_char(c);

        if c_is_chinese {
            if !current.is_empty() && !current_is_chinese {
                words.push(current.clone());
                current.clear();
            }
            current.push(c);
            words.push(current.clone());
            current.clear();
            current_is_chinese = false;
        } else if is_delimiter(c) {
            if !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
            current_is_chinese = false;
        } else {
            if !current.is_empty() && current_is_chinese {
                words.push(current.clone());
                current.clear();
            }
            current.push(c);
            current_is_chinese = false;
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    let words = split_into_words(text);
    if words.is_empty() {
        return vec![];
    }

    let tokens: Vec<f64> = words.iter().map(|w| estimate_tokens(w)).collect();

    let mut chunks = Vec::new();
    let mut start_idx = 0usize;

    while start_idx < words.len() {
        let mut end_idx = start_idx;
        let mut current_tokens = 0.0f64;

        while end_idx < words.len() {
            let next_tokens = current_tokens + tokens[end_idx];
            if next_tokens > chunk_size as f64 && end_idx > start_idx {
                break;
            }
            current_tokens = next_tokens;
            end_idx += 1;
        }

        if end_idx == start_idx {
            end_idx = start_idx + 1;
        }

        let chunk = words[start_idx..end_idx].join("");
        chunks.push(chunk);

        if end_idx >= words.len() {
            break;
        }

        let mut overlap_tokens = 0.0f64;
        let mut new_start = end_idx;

        while new_start > start_idx && overlap_tokens < overlap as f64 {
            new_start -= 1;
            overlap_tokens += tokens[new_start];
        }

        if new_start <= start_idx {
            start_idx = end_idx;
        } else {
            start_idx = new_start;
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(chunk_text("", 100, 20), Vec::<String>::new());
    }

    #[test]
    fn test_simple_english() {
        let text = "Hello world this is a test of the chunking system for simple English text processing.";
        let chunks = chunk_text(text, 10, 2);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chinese() {
        let text = "这是一段中文测试文本，用于验证分块系统的正确性。我们希望它能够正确处理中文字符。";
        let chunks = chunk_text(text, 10, 2);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_mixed() {
        let text = "这是一段混合了English和中文的文本mix测试。This is a mixed language test. 包含中文和英文。";
        let chunks = chunk_text(text, 15, 3);
        assert!(!chunks.is_empty());
    }
}