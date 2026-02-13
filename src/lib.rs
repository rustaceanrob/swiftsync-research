use kernel::BlockTreeEntry;

const REFERENCE_HEIGHT: i32 = 930_000;

pub fn is_reference_height(entry: BlockTreeEntry) -> bool {
    entry.height() == 930_000
}
