use crate::BlockLayout;

/// 一次导航移动请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// 移动到下一文本块的首句。
    NextBlock,
    /// 移动到上一文本块的首句。
    PreviousBlock,
    /// 移动到下一句。
    NextSentence,
    /// 移动到上一句。
    PreviousSentence,
}

/// 当前活动句子及文本块布局。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigation {
    layout: BlockLayout,
    active_sentence: usize,
}

impl Navigation {
    /// 创建导航状态，并将首个文本块的首句设为活动句子。
    ///
    /// 句子数为 0 时返回 `None`。
    pub fn new(layout: BlockLayout) -> Option<Self> {
        if layout.sentence_count() == 0 {
            return None;
        }
        let active_sentence = layout
            .blocks()
            .iter()
            .find_map(|block| block.first_sentence())
            .unwrap_or(0);
        Some(Self {
            layout,
            active_sentence,
        })
    }

    /// 返回当前活动句子的索引。
    pub fn active_sentence(&self) -> usize {
        self.active_sentence
    }

    /// 返回当前文本块布局。
    pub fn layout(&self) -> &BlockLayout {
        &self.layout
    }

    /// 返回活动句子所属文本块的索引。
    pub fn current_block(&self) -> Option<usize> {
        self.layout.block_of(self.active_sentence)
    }

    /// 应用一次移动。
    ///
    /// 活动句子发生变化时返回 `true`。
    pub fn apply(&mut self, motion: Motion) -> bool {
        let next = match motion {
            Motion::NextBlock => self.next_block_target(),
            Motion::PreviousBlock => self.previous_block_target(),
            Motion::NextSentence => self.next_sentence_target(),
            Motion::PreviousSentence => self.previous_sentence_target(),
        };
        match next {
            Some(next) if next != self.active_sentence => {
                self.active_sentence = next;
                true
            }
            _ => false,
        }
    }

    /// 设置活动句子。
    ///
    /// 索引越界时返回 `false`，状态不变。
    pub fn set_active(&mut self, sentence_index: usize) -> bool {
        if sentence_index >= self.layout.sentence_count() {
            return false;
        }
        self.active_sentence = sentence_index;
        true
    }

    /// 用新的文本块布局重建状态，并保留原活动索引。
    ///
    /// 新布局没有句子时返回 `false`。
    pub fn reflow(&mut self, layout: BlockLayout) -> bool {
        if layout.sentence_count() == 0 {
            return false;
        }
        self.active_sentence = self
            .active_sentence
            .min(layout.sentence_count().saturating_sub(1));
        self.layout = layout;
        true
    }

    fn next_block_target(&self) -> Option<usize> {
        let current = self.current_block()?;
        self.layout
            .blocks()
            .get(current + 1)
            .and_then(|block| block.first_sentence())
    }

    fn previous_block_target(&self) -> Option<usize> {
        let current = self.current_block()?;
        current
            .checked_sub(1)
            .and_then(|block_index| self.layout.block(block_index))
            .and_then(|block| block.first_sentence())
    }

    fn next_sentence_target(&self) -> Option<usize> {
        let next = self.active_sentence.checked_add(1)?;
        (next < self.layout.sentence_count()).then_some(next)
    }

    fn previous_sentence_target(&self) -> Option<usize> {
        self.active_sentence.checked_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn navigation() -> Navigation {
        let layout = BlockLayout::from_blocks(5, vec![vec![0, 1], vec![2, 3], vec![4]]);
        Navigation::new(layout).unwrap()
    }

    #[test]
    fn j_and_k_move_between_block_starts() {
        let mut navigation = navigation();
        assert!(navigation.apply(Motion::NextBlock));
        assert_eq!(navigation.active_sentence(), 2);
        assert!(navigation.apply(Motion::NextBlock));
        assert_eq!(navigation.active_sentence(), 4);
        assert!(navigation.apply(Motion::PreviousBlock));
        assert_eq!(navigation.active_sentence(), 2);
    }

    #[test]
    fn h_and_l_cross_block_boundaries_by_sentence() {
        let mut navigation = navigation();
        assert!(navigation.apply(Motion::NextSentence));
        assert_eq!(navigation.active_sentence(), 1);
        assert!(navigation.apply(Motion::NextSentence));
        assert_eq!(navigation.active_sentence(), 2);
        assert!(navigation.apply(Motion::PreviousSentence));
        assert_eq!(navigation.active_sentence(), 1);
    }

    #[test]
    fn navigation_stops_at_document_boundaries() {
        let mut navigation = navigation();
        assert!(!navigation.apply(Motion::PreviousSentence));
        assert!(!navigation.apply(Motion::PreviousBlock));
        assert_eq!(navigation.active_sentence(), 0);

        for _ in 0..10 {
            navigation.apply(Motion::NextSentence);
        }
        assert_eq!(navigation.active_sentence(), 4);
        assert!(!navigation.apply(Motion::NextSentence));
        assert!(!navigation.apply(Motion::NextBlock));
    }

    #[test]
    fn reflow_keeps_the_active_index_when_possible() {
        let mut navigation = navigation();
        navigation.apply(Motion::NextSentence);
        let smaller = BlockLayout::from_blocks(1, vec![vec![0]]);
        assert!(navigation.reflow(smaller));
        assert_eq!(navigation.active_sentence(), 0);
    }

    #[test]
    fn set_active_rejects_an_out_of_range_index() {
        let mut navigation = navigation();
        assert!(navigation.set_active(3));
        assert_eq!(navigation.active_sentence(), 3);
        assert!(!navigation.set_active(5));
        assert_eq!(navigation.active_sentence(), 3);
    }
}
