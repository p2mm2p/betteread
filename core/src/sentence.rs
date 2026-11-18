use icu_segmenter::SentenceSegmenter;

/// UTF-16 码元中的句子边界。
///
/// `start` 包含，`end` 不包含。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SentenceRange {
    /// 起始偏移，包含。
    pub start: u32,
    /// 结束偏移，不包含。
    pub end: u32,
}

impl SentenceRange {
    /// 创建句子范围。
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// 返回范围的 UTF-16 长度。
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// 返回范围是否为空。
    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// 切分文本并去除句子首尾空白。
///
/// 返回的偏移以 UTF-16 码元计。空文本和纯空白文本返回空列表。
///
/// # Examples
///
/// ```
/// use brcore::split_sentences;
///
/// let ranges = split_sentences("Hello. World!");
/// assert_eq!(ranges.len(), 2);
/// ```
pub fn split_sentences(text: &str) -> Vec<SentenceRange> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let utf16: Vec<u16> = text.encode_utf16().collect();
    let segmenter = SentenceSegmenter::new(Default::default());
    let mut breaks = segmenter.segment_utf16(&utf16);
    let Some(mut start) = breaks.next() else {
        return Vec::new();
    };

    let mut sentences = Vec::new();
    for end in breaks {
        let (trimmed_start, trimmed_end) = trim_utf16_range(&utf16[start..end]);
        if trimmed_start < trimmed_end {
            sentences.push(SentenceRange::new(
                (start + trimmed_start) as u32,
                (start + trimmed_end) as u32,
            ));
        }
        start = end;
    }
    sentences
}

fn trim_utf16_range(input: &[u16]) -> (usize, usize) {
    let mut first_visible = None;
    let mut last_visible_end = 0;
    let mut position = 0;

    for decoded in char::decode_utf16(input.iter().copied()) {
        let ch = decoded.unwrap_or(char::REPLACEMENT_CHARACTER);
        let width = ch.len_utf16();
        if !ch.is_whitespace() {
            first_visible.get_or_insert(position);
            last_visible_end = position + width;
        }
        position += width;
    }

    match first_visible {
        Some(start) => (start, last_visible_end),
        None => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_slice(text: &str, range: SentenceRange) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[range.start as usize..range.end as usize]).unwrap()
    }

    #[test]
    fn splits_english_sentences_and_trims_spaces() {
        let text = "First sentence.  Second sentence!";
        let sentences = split_sentences(text);
        assert_eq!(
            sentences
                .iter()
                .map(|range| utf16_slice(text, *range))
                .collect::<Vec<_>>(),
            ["First sentence.", "Second sentence!"]
        );
        assert_eq!(sentences[0], SentenceRange::new(0, 15));
        assert_eq!(sentences[1].start, 17);
    }

    #[test]
    fn splits_chinese_sentences() {
        let text = "今天很好。明天见！";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 2);
        assert_eq!(utf16_slice(text, sentences[0]), "今天很好。");
        assert_eq!(utf16_slice(text, sentences[1]), "明天见！");
    }

    #[test]
    fn keeps_emoji_offsets_in_utf16_units() {
        let text = "Hi 👋. Bye!";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0], SentenceRange::new(0, 6));
        assert_eq!(sentences[1], SentenceRange::new(7, 11));
        assert_eq!(utf16_slice(text, sentences[0]), "Hi 👋.");
    }

    #[test]
    fn empty_and_whitespace_text_have_no_sentences() {
        assert!(split_sentences("").is_empty());
        assert!(split_sentences(" \n\t ").is_empty());
    }
}
