use brcore::{BlockLayout, HIGHLIGHT_NAME, SentenceRange, highlight_css, split_sentences};
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, HtmlStyleElement, Node, Range, Text, Window};

const STYLE_ID: &str = "betteread-highlight-style";
const BLOCK_TAGS: &[&str] = &[
    "caption",
    "blockquote",
    "dd",
    "dt",
    "figcaption",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "p",
    "pre",
    "summary",
    "td",
    "th",
];
const SKIPPED_TAGS: &[&str] = &[
    "base", "button", "head", "input", "link", "meta", "noscript", "option", "script", "select",
    "style", "svg", "template", "textarea", "title",
];

/// 页面文本扫描结果。
pub struct PageSnapshot {
    /// 按文档顺序排列的可读句子。
    pub sentences: Vec<PageSentence>,
    /// 句子所属语义文本块构成的导航布局。
    pub blocks: BlockLayout,
}

/// 页面中的一个可读句子。
pub struct PageSentence {
    /// 句子文本。
    pub text: String,
    /// 句子对应的 DOM Range。
    pub range: Range,
}

/// 扫描页面的可读文本并构建句子与文本块模型。
///
/// 页面没有可读句子时返回 `Ok(None)`。
///
/// # Errors
///
/// DOM 扫描或矩形读取失败时返回错误文本。
pub fn scan(document: &Document) -> Result<Option<PageSnapshot>, String> {
    let Some(body) = document.body() else {
        return Ok(None);
    };

    let mut block_texts = Vec::new();
    collect_node(body.as_ref(), &mut block_texts, None);

    let mut sentences = Vec::new();
    let mut block_sentence_lists = Vec::new();

    for block in block_texts {
        if block.text.trim().is_empty() {
            continue;
        }

        let ranges = split_sentences(&block.text);
        if ranges.is_empty() {
            continue;
        }

        let utf16: Vec<u16> = block.text.encode_utf16().collect();
        let mut sentence_indices = Vec::new();

        for range in ranges {
            let Some(dom_range) = block.dom_range(document, range) else {
                continue;
            };
            if !has_visible_rects(&dom_range) {
                continue;
            }

            let text = String::from_utf16_lossy(&utf16[range.start as usize..range.end as usize]);
            let sentence_index = sentences.len();
            sentences.push(PageSentence {
                text,
                range: dom_range,
            });
            sentence_indices.push(sentence_index);
        }

        if !sentence_indices.is_empty() {
            block_sentence_lists.push(sentence_indices);
        }
    }

    if sentences.is_empty() {
        return Ok(None);
    }

    let blocks = BlockLayout::from_blocks(sentences.len(), block_sentence_lists);
    Ok(Some(PageSnapshot { sentences, blocks }))
}

/// 安装活动句子的高亮样式。
///
/// # Errors
///
/// 无法创建或插入样式元素时返回错误文本。
pub fn install_style(
    document: &Document,
    settings: &brcore::ReadingSettings,
) -> Result<HtmlStyleElement, String> {
    if let Some(existing) = document.get_element_by_id(STYLE_ID) {
        existing.remove();
    }
    let style = document
        .create_element("style")
        .map_err(js_error)?
        .dyn_into::<HtmlStyleElement>()
        .map_err(|_| "could not create the highlight style element".to_string())?;
    style.set_id(STYLE_ID);
    style.set_text_content(Some(&highlight_css(settings)));
    document
        .document_element()
        .ok_or_else(|| "page has no document element".to_string())?
        .append_child(style.as_ref())
        .map_err(js_error)?;
    Ok(style)
}

/// CSS Custom Highlight 的页面适配器。
pub struct HighlightHandle {
    window: Window,
    highlights: JsValue,
}

impl HighlightHandle {
    /// 创建高亮适配器。
    ///
    /// # Errors
    ///
    /// 浏览器不支持 CSS Custom Highlight 时返回错误文本。
    pub fn new(window: &Window) -> Result<Self, String> {
        let constructor =
            Reflect::get(window.as_ref(), &JsValue::from_str("Highlight")).map_err(js_error)?;
        if !constructor.is_function() {
            return Err("this browser does not support CSS Custom Highlight".to_string());
        }
        let css = Reflect::get(window.as_ref(), &JsValue::from_str("CSS")).map_err(js_error)?;
        let highlights = Reflect::get(&css, &JsValue::from_str("highlights")).map_err(js_error)?;
        if highlights.is_undefined() || highlights.is_null() {
            return Err("this browser does not expose CSS.highlights".to_string());
        }
        Ok(Self {
            window: window.clone(),
            highlights,
        })
    }

    /// 将高亮设置为指定 Range。
    ///
    /// # Errors
    ///
    /// 注册高亮失败时返回错误文本。
    pub fn show(&self, range: &Range) -> Result<(), String> {
        let constructor = Reflect::get(self.window.as_ref(), &JsValue::from_str("Highlight"))
            .map_err(js_error)?;
        let constructor: Function = constructor
            .dyn_into()
            .map_err(|_| "Highlight is not a constructor".to_string())?;
        let args = Array::new();
        args.push(range.as_ref());
        let highlight = Reflect::construct(&constructor, &args).map_err(js_error)?;
        let set = Reflect::get(&self.highlights, &JsValue::from_str("set")).map_err(js_error)?;
        let set: Function = set
            .dyn_into()
            .map_err(|_| "CSS.highlights.set is unavailable".to_string())?;
        set.call2(
            &self.highlights,
            &JsValue::from_str(HIGHLIGHT_NAME),
            &highlight,
        )
        .map_err(js_error)?;
        Ok(())
    }

    /// 移除活动高亮。
    pub fn clear(&self) {
        if let Ok(delete) = Reflect::get(&self.highlights, &JsValue::from_str("delete"))
            && let Ok(delete) = delete.dyn_into::<Function>()
        {
            let _ = delete.call1(&self.highlights, &JsValue::from_str(HIGHLIGHT_NAME));
        }
    }
}

/// 活动句子不在视口内时滚动到可见位置。
pub fn ensure_visible(window: &Window, range: &Range) {
    let Some(rects) = range.get_client_rects() else {
        return;
    };
    let Some(rect) = rects.item(0) else {
        return;
    };
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    if rect.top() >= 0.0 && rect.bottom() <= viewport_height {
        return;
    }
    let Some(scroll_to) = Reflect::get(window.as_ref(), &JsValue::from_str("scrollTo"))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
    else {
        return;
    };
    let options = Object::new();
    let top = window.scroll_y().unwrap_or(0.0) + rect.top() - viewport_height * 0.25;
    let _ = Reflect::set(&options, &JsValue::from_str("top"), &JsValue::from_f64(top));
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("behavior"),
        &JsValue::from_str("smooth"),
    );
    let _ = scroll_to.call1(window.as_ref(), options.as_ref());
}

fn has_visible_rects(range: &Range) -> bool {
    let Some(rects) = range.get_client_rects() else {
        return false;
    };
    (0..rects.length()).any(|index| {
        rects
            .item(index)
            .is_some_and(|rect| rect.width() > 0.0 && rect.height() > 0.0)
    })
}

fn is_block(tag: &str) -> bool {
    BLOCK_TAGS.contains(&tag)
}

fn is_skipped(tag: &str) -> bool {
    SKIPPED_TAGS.contains(&tag)
}

fn collect_node(node: &Node, blocks: &mut Vec<BlockText>, current_block: Option<usize>) {
    match node.node_type() {
        Node::TEXT_NODE => {
            let Some(block_index) = current_block else {
                return;
            };
            if let Ok(text) = node.clone().dyn_into::<Text>() {
                blocks[block_index].push_text(text);
            }
        }
        Node::ELEMENT_NODE => {
            let Ok(element) = node.clone().dyn_into::<Element>() else {
                return;
            };
            let tag = element.tag_name().to_ascii_lowercase();
            if is_skipped(&tag)
                || element.has_attribute("hidden")
                || element.get_attribute("aria-hidden").as_deref() == Some("true")
            {
                return;
            }
            if tag == "br" {
                if let Some(block_index) = current_block {
                    blocks[block_index].separator();
                }
                return;
            }
            if is_block(&tag) {
                blocks.push(BlockText::default());
                let block_index = blocks.len() - 1;
                collect_children(node, blocks, Some(block_index));
                return;
            }
            collect_children(node, blocks, current_block);
        }
        _ => collect_children(node, blocks, current_block),
    }
}

fn collect_children(node: &Node, blocks: &mut Vec<BlockText>, current_block: Option<usize>) {
    let mut child = node.first_child();
    while let Some(current) = child {
        collect_node(&current, blocks, current_block);
        child = current.next_sibling();
    }
}

#[derive(Default)]
struct BlockText {
    text: String,
    len_utf16: usize,
    parts: Vec<TextPart>,
}

struct TextPart {
    start: usize,
    len: usize,
    node: Option<Text>,
}

impl BlockText {
    fn push_text(&mut self, node: Text) {
        let data = node.data();
        if data.is_empty() {
            return;
        }
        let len = data.encode_utf16().count();
        self.parts.push(TextPart {
            start: self.len_utf16,
            len,
            node: Some(node),
        });
        self.text.push_str(&data);
        self.len_utf16 += len;
    }

    fn separator(&mut self) {
        if self
            .text
            .chars()
            .last()
            .is_some_and(|character| !character.is_whitespace())
        {
            self.parts.push(TextPart {
                start: self.len_utf16,
                len: 1,
                node: None,
            });
            self.text.push('\n');
            self.len_utf16 += 1;
        }
    }

    fn dom_range(&self, document: &Document, range: SentenceRange) -> Option<Range> {
        let start = range.start as usize;
        let end = range.end as usize;
        if start >= end {
            return None;
        }
        let start_part = self
            .parts
            .iter()
            .find(|part| part.node.is_some() && part.start + part.len > start)?;
        let end_part = self
            .parts
            .iter()
            .rev()
            .find(|part| part.node.is_some() && part.start < end)?;
        let start_node = start_part.node.as_ref()?;
        let end_node = end_part.node.as_ref()?;
        let start_offset = start.saturating_sub(start_part.start).min(start_part.len) as u32;
        let end_offset = end.saturating_sub(end_part.start).min(end_part.len) as u32;
        let dom_range = document.create_range().ok()?;
        dom_range
            .set_start(start_node.as_ref(), start_offset)
            .ok()?;
        dom_range.set_end(end_node.as_ref(), end_offset).ok()?;
        Some(dom_range)
    }
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_blocks_and_inline_tags_are_classified() {
        assert!(is_block("p"));
        assert!(is_block("h2"));
        assert!(is_block("li"));
        assert!(is_block("td"));
        assert!(!is_block("div"));
        assert!(!is_block("section"));
        assert!(!is_block("span"));
        assert!(!is_block("strong"));
        assert!(is_skipped("script"));
        assert!(is_skipped("textarea"));
        assert!(!is_skipped("p"));
    }
}
