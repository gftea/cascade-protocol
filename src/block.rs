use crate::{key::Key, shuffled_key::ShuffledKey};
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

type WeakBlockRef = Weak<Block>;
pub type BlockRef = Rc<Block>;
/// A block is a contiguous range of bits in a shuffled key.
/// not thread safe, it use RefCell to allow interior mutability
#[derive(Debug)]
pub struct Block {
    inner: RefCell<Inner>,
}

#[derive(Debug)]
struct Inner {
    // the type of the block
    block_type: BlockType,

    // start index of in the shuffled key
    start_bit_nr: u32,
    // end index of in the shuffled key
    end_bit_nr: u32,
    // reference to the shuffled key
    shuffled_key: ShuffledKey,
    // the parity of the bits in the range
    current_parity: Option<u8>,
    // the parity answerd by the remote
    correct_parity: Option<u8>,
    // the parent block
    parent: Option<WeakBlockRef>,
    // the left sub block
    left_sub_block: Option<BlockRef>,
    // the right sub block
    right_sub_block: Option<BlockRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    TopLevel,
    SubBlock(SubBlockType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubBlockType {
    Left,
    Right,
}
impl Inner {
    fn new(
        block_type: BlockType,
        start_bit_nr: u32,
        end_bit_nr: u32,
        shuffled_key: ShuffledKey,
    ) -> Self {
        Self {
            block_type,
            start_bit_nr,
            end_bit_nr,
            shuffled_key,
            current_parity: None,
            correct_parity: None,
            parent: None,
            left_sub_block: None,
            right_sub_block: None,
        }
    }
}

/// Block only provide API for working with Rc<Block>
impl Block {
    /// The range is inclusive, a.k.a `start_bit_nr..=end_bit_nr the`
    pub fn new(
        block_type: BlockType,
        start_bit_nr: u32,
        end_bit_nr: u32,
        shuffled_key: ShuffledKey,
    ) -> BlockRef {
        let inner = Inner::new(block_type, start_bit_nr, end_bit_nr, shuffled_key);
        let block = Rc::new(Block {
            inner: RefCell::new(inner),
        });
        block
    }

    pub fn get_block_type(&self) -> BlockType {
        self.inner.borrow().block_type.clone()
    }
    pub fn get_start_bit_nr(&self) -> u32 {
        self.inner.borrow().start_bit_nr
    }

    pub fn get_end_bit_nr(&self) -> u32 {
        self.inner.borrow().end_bit_nr
    }

    pub fn contains_bit(&self, bit_nr: u32) -> bool {
        self.get_start_bit_nr() <= bit_nr && bit_nr <= self.get_end_bit_nr()
    }

    pub fn get_nr_bits(&self) -> u32 {
        self.inner.borrow().end_bit_nr - self.inner.borrow().start_bit_nr + 1
    }

    pub fn get_correct_parity(&self) -> Option<u8> {
        self.inner.borrow().correct_parity
    }

    pub fn get_or_compute_current_parity(&self) -> u8 {
        let current_parity = self.inner.borrow().current_parity;
        match current_parity {
            Some(parity) => parity,
            None => {
                println!("compute_current_parity {}", self);
                let start_bit_nr = self.inner.borrow().start_bit_nr;
                let end_bit_nr = self.inner.borrow().end_bit_nr;
                let parity = self
                    .inner
                    .borrow()
                    .shuffled_key
                    .compute_range_parity(start_bit_nr, end_bit_nr);
                self.inner.borrow_mut().current_parity = Some(parity);
                parity
            }
        }
    }
    pub fn correct_bit(&self, bit_nr: u32) {
        self.inner.borrow_mut().shuffled_key.flip_bit(bit_nr);
    }
    pub fn flip_current_parity(&self) {
        if self.inner.borrow().current_parity.is_none() {
            println!("current_parity is unknown, skip flip block {} ", self);
            return;
        }
        println!("flip_current_parity { }", self);
        let current_parity = self.inner.borrow().current_parity.unwrap();

        self.inner.borrow_mut().current_parity = Some(1 - current_parity);
    }

    pub fn set_correct_parity(&self, correct_parity: u8) {
        self.inner.borrow_mut().correct_parity = Some(correct_parity);
    }

    /// Error Parity.
    /// # Panics
    ///
    /// Panics if `correct_parity` is not known
    ///
    /// # Returns
    ///
    /// `true`: Odd number of errors in block,
    /// `false`: Even number of errors in block
    pub fn get_error_parity(&self) -> bool {
        let correct_parity = self
            .inner
            .borrow()
            .correct_parity
            .expect("correct_parity must be known");
        let current_parity = self.get_or_compute_current_parity();
        let error_parity = current_parity != correct_parity;
        // println!("get_error_parity: {}, block: {} ", error_parity, self);
        error_parity
    }

    pub fn get_parent_block(&self) -> Option<BlockRef> {
        match self.inner.borrow().parent {
            Some(ref weak_parent) => {
                let parent = weak_parent.upgrade().expect("Parent block must exist");
                Some(parent)
            }
            None => None,
        }
    }

    pub fn get_left_sub_block(&self) -> Option<BlockRef> {
        self.inner.borrow().left_sub_block.clone()
    }

    pub fn get_right_sub_block(&self) -> Option<BlockRef> {
        self.inner.borrow().right_sub_block.clone()
    }

    pub fn has_sub_blocks(&self) -> bool {
        // either both are None or both are Some
        assert!(self.get_left_sub_block().is_some() == self.get_right_sub_block().is_some());
        self.get_left_sub_block().is_some() && self.get_right_sub_block().is_some()
    }

    /// Create a new sub block and set the parent block to self
    ///
    /// # Arguments
    /// left: if true, create a left sub block, otherwise create a right sub block
    pub fn create_sub_block(self: &BlockRef, left: SubBlockType) -> BlockRef {
        let start_bit_nr = self.inner.borrow().start_bit_nr;
        let end_bit_nr = self.inner.borrow().end_bit_nr;
        let mid_bit_nr = (start_bit_nr + end_bit_nr) / 2;
        let sub_block = match left {
            SubBlockType::Left => {
                let block = Block::new(
                    BlockType::SubBlock(SubBlockType::Left),
                    start_bit_nr,
                    mid_bit_nr,
                    self.inner.borrow().shuffled_key.clone(),
                );
                self.inner.borrow_mut().left_sub_block = Some(block.clone());
                block
            }
            SubBlockType::Right => {
                let block = Block::new(
                    BlockType::SubBlock(SubBlockType::Right),
                    mid_bit_nr + 1,
                    end_bit_nr,
                    self.inner.borrow().shuffled_key.clone(),
                );
                self.inner.borrow_mut().right_sub_block = Some(block.clone());
                block
            }
        };
        // set parent for sub block
        let parent_block = Rc::downgrade(self);
        sub_block.inner.borrow_mut().parent = Some(parent_block);
        sub_block
    }

    /// Try to infer the correct parity of the block.
    ///
    /// # Returns
    ///
    /// `true`: correct parity was inferred, otherwise `false`
    pub fn try_to_infer_correct_parity(self: &BlockRef) -> bool {
        // only try to infer if correct_parity is not known yet
        if self.get_correct_parity().is_some() {
            return true;
        }

        // Cannot infer if there is no parent block.
        if self.inner.borrow().parent.is_none() {
            return false;
        }

        let parent_block = self
            .inner
            .borrow()
            .parent
            .as_ref()
            .unwrap()
            .upgrade()
            .expect("parent block must exist");

        // Cannot infer if there is no sibling block (yet).
        if parent_block.get_left_sub_block().is_none()
            || parent_block.get_right_sub_block().is_none()
        {
            return false;
        }

        // Cannot infer if the correct parity of the parent is unknown.
        if parent_block.get_correct_parity().is_none() {
            return false;
        }

        // get sibling block
        let mut sibling_block = parent_block.get_left_sub_block().unwrap();

        // check equality of pointers to determine if self is the left or right sub block
        if Rc::ptr_eq(self, &sibling_block) {
            // if self is the left sub block, get the right sub block
            sibling_block = parent_block.get_right_sub_block().unwrap();
        }
        // Cannot infer if the correct parity of the sibling is unknown.
        if sibling_block.get_correct_parity().is_none() {
            return false;
        }
        // XOR the correct parities of the parent and sibling block to get the correct parity of this block
        let inferred_correct_parity = parent_block.get_correct_parity().unwrap()
            ^ sibling_block.get_correct_parity().unwrap();
        self.inner.borrow_mut().correct_parity = Some(inferred_correct_parity);
        true
    }

    // simulate asking the correct parity of the block
    // calculate correct parity using original correct key
    pub fn ask_correct_parity(self: &BlockRef) {
        if self.get_correct_parity().is_some() {
            println!("Correct parity already known: {}", self);
            return;
        }
        println!("Ask correct parity: {}", self);

        let correct_parity = self
            .inner
            .borrow()
            .shuffled_key
            .ask_correct_range_parity(self.get_start_bit_nr(), self.get_end_bit_nr());
        self.set_correct_parity(correct_parity);
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} ({}-{})",
            self.get_block_type(),
            self.get_start_bit_nr(),
            self.get_end_bit_nr()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        block::{Block, BlockRef, BlockType, SubBlockType},
        key::Key,
        shuffle,
        shuffled_key::{SharedKey, ShuffledKey},
    };
    use std::{cell::RefCell, mem, rc::Rc};

    fn create_test_shuffled_key() -> (BlockRef, SharedKey) {
        const SEED: u64 = 0x1234567890ABCDEF;
        const KEY_STR: &str = "10010001";
        let correct_key = Key::from(KEY_STR);
        let key = Rc::new(RefCell::new(correct_key.clone()));
        let shuffle =
            shuffle::Shuffle::new_shuffle_from_seed(1, key.borrow().get_nr_bits(), SEED, true);
        let top_block_start_bit_nr = 0;
        let top_block_end_bit_nr = 3;
        let block = Block::new(
            BlockType::TopLevel,
            top_block_start_bit_nr,
            top_block_end_bit_nr,
            ShuffledKey::new(Rc::new(correct_key), key.clone(), shuffle),
        );

        (block, key)
    }

    #[test]
    fn test_apis_with_block_receiver() {
        let (block, key) = create_test_shuffled_key();

        assert_eq!(block.get_start_bit_nr(), 0);
        assert_eq!(block.get_end_bit_nr(), 3);
        assert_eq!(block.get_nr_bits(), 4);
        assert_eq!(block.get_block_type(), BlockType::TopLevel);
        assert_eq!(mem::size_of::<BlockType>(), 1);
        assert!(block.get_correct_parity().is_none());
        assert!(block.get_parent_block().is_none());
        assert!(block.get_left_sub_block().is_none());
        assert!(block.get_right_sub_block().is_none());

        // parity is not known yet, so it should be computed
        assert_eq!(block.get_or_compute_current_parity(), 0);
        // flip the parity after a single bit in Key is corrected, so that we do not need to recompute the parity
        // Note: this is not the correct way to correct a bit in Key, but it is sufficient for testing
        key.borrow_mut().flip_bit(0);
        assert_eq!(
            key.borrow()
                .compute_range_parity(block.get_start_bit_nr(), block.get_end_bit_nr()),
            1
        );
        block.flip_current_parity();
        assert_eq!(block.get_or_compute_current_parity(), 1);

        // set correct_parity, assume we got it from the remote
        block.set_correct_parity(0);
        assert_eq!(block.get_correct_parity(), Some(0));
        // check error parity
        assert_eq!(block.get_error_parity(), true);
    }

    #[test]
    pub fn test_apis_with_rc_receiver() {
        let (top_block, _) = create_test_shuffled_key();
        let block2 = top_block.clone();
        let left_sub_block = top_block.create_sub_block(SubBlockType::Left);
        let right_sub_block = top_block.create_sub_block(SubBlockType::Right);
        // two Rc: top_block + block2
        assert_eq!(Rc::strong_count(&top_block), 2);
        // two Rc: `left_sub_block` + the one insdie `top_block`
        assert_eq!(Rc::strong_count(&left_sub_block), 2);
        // two Rc: `right_sub_block` + the one insdie `top_block`
        assert_eq!(Rc::strong_count(&right_sub_block), 2);

        // move the blocks into a closure and drop them
        #[allow(path_statements)]
        let f = move || {
            top_block;
            left_sub_block;
            right_sub_block;
        };
        f();

        assert_eq!(Rc::strong_count(&block2), 1);
    }

    #[test]
    pub fn test_infer_correct_parity() {
        let (top_block, _) = create_test_shuffled_key();
        let left_sub_block = top_block.create_sub_block(SubBlockType::Left);
        let right_sub_block = top_block.create_sub_block(SubBlockType::Right);

        // cannot infer if there is no parent block
        assert_eq!(top_block.try_to_infer_correct_parity(), false);

        // cannot infer if the correct parity of the parent is unknown
        assert_eq!(left_sub_block.try_to_infer_correct_parity(), false);
        assert_eq!(right_sub_block.try_to_infer_correct_parity(), false);

        // set correct_parity, assume we got it from the remote
        top_block.set_correct_parity(0);
        assert_eq!(top_block.get_correct_parity(), Some(0));

        // cannot infer if the correct parity of the sibling is unknown
        assert_eq!(left_sub_block.try_to_infer_correct_parity(), false);
        assert_eq!(right_sub_block.try_to_infer_correct_parity(), false);

        // set correct_parity, assume we got it from the remote
        left_sub_block.set_correct_parity(0);
        assert_eq!(left_sub_block.get_correct_parity(), Some(0));

        // XOR the correct parities of the parent and sibling block to get the correct parity of this block
        assert_eq!(right_sub_block.try_to_infer_correct_parity(), true);
        assert_eq!(right_sub_block.get_correct_parity(), Some(0));
    }
}
