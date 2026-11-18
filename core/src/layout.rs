/// 一个语义文本块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    /// 该块中的句子索引，按阅读顺序排列。
    pub sentence_indices: Vec<usize>,
}

impl TextBlock {
    /// 返回该块的首句索引。
    pub fn first_sentence(&self) -> Option<usize> {
        self.sentence_indices.first().copied()
    }
}

/// 按文档顺序排列的文本块模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockLayout {
    blocks: Vec<TextBlock>,
    sentence_blocks: Vec<Option<usize>>,
}

impl BlockLayout {
    /// 根据每个块包含的句子索引构建模型。
    ///
    /// `blocks` 的顺序就是阅读顺序。空块、越界索引和重复索引会被忽略。
    pub fn from_blocks(sentence_count: usize, blocks: Vec<Vec<usize>>) -> Self {
        let mut sentence_blocks = vec![None; sentence_count];
        let mut cleaned_blocks = Vec::new();

        for block in blocks {
            let mut indices = Vec::new();

            for sentence_index in block {
                if sentence_index >= sentence_count {
                    continue;
                }
                if indices.contains(&sentence_index) {
                    continue;
                }
                if sentence_blocks[sentence_index].is_some() {
                    continue;
                }
                indices.push(sentence_index);
            }

            if indices.is_empty() {
                continue;
            }

            indices.sort_unstable();
            let block_index = cleaned_blocks.len();

            for &sentence_index in &indices {
                sentence_blocks[sentence_index] = Some(block_index);
            }

            cleaned_blocks.push(TextBlock {
                sentence_indices: indices,
            });
        }

        Self {
            blocks: cleaned_blocks,
            sentence_blocks,
        }
    }

    /// 返回按阅读顺序排列的文本块。
    pub fn blocks(&self) -> &[TextBlock] {
        &self.blocks
    }

    /// 返回指定索引的文本块。
    pub fn block(&self, block_index: usize) -> Option<&TextBlock> {
        self.blocks.get(block_index)
    }

    /// 返回句子所属文本块的索引。
    pub fn block_of(&self, sentence_index: usize) -> Option<usize> {
        self.sentence_blocks.get(sentence_index).copied().flatten()
    }

    /// 返回句子总数。
    pub fn sentence_count(&self) -> usize {
        self.sentence_blocks.len()
    }

    /// 返回布局是否没有可用文本块。
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_sentences(layout: &BlockLayout) -> Vec<Vec<usize>> {
        layout
            .blocks()
            .iter()
            .map(|block| block.sentence_indices.clone())
            .collect()
    }

    #[test]
    fn builds_blocks_in_document_order() {
        let layout = BlockLayout::from_blocks(5, vec![vec![0, 1], vec![2], vec![3, 4]]);
        assert_eq!(
            block_sentences(&layout),
            vec![vec![0, 1], vec![2], vec![3, 4]]
        );
        assert_eq!(layout.block_of(0), Some(0));
        assert_eq!(layout.block_of(2), Some(1));
        assert_eq!(layout.block_of(4), Some(2));
    }

    #[test]
    fn drops_empty_blocks_and_invalid_sentences() {
        let layout = BlockLayout::from_blocks(3, vec![vec![], vec![1, 1, 9], vec![0], vec![2]]);
        assert_eq!(block_sentences(&layout), vec![vec![1], vec![0], vec![2]]);
    }

    #[test]
    fn keeps_a_sentence_in_its_first_block() {
        let layout = BlockLayout::from_blocks(2, vec![vec![0, 1], vec![1]]);
        assert_eq!(block_sentences(&layout), vec![vec![0, 1]]);
        assert_eq!(layout.block_of(1), Some(0));
    }

    #[test]
    fn empty_layout_has_no_blocks() {
        let layout = BlockLayout::from_blocks(0, vec![vec![0], vec![1]]);
        assert!(layout.is_empty());
        assert_eq!(layout.sentence_count(), 0);
    }

    #[test]
    fn block_exposes_its_first_sentence() {
        let block = TextBlock {
            sentence_indices: vec![4, 5],
        };
        assert_eq!(block.first_sentence(), Some(4));

        let empty = TextBlock {
            sentence_indices: Vec::new(),
        };
        assert_eq!(empty.first_sentence(), None);
    }
}
