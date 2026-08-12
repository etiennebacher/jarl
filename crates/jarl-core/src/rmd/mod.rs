pub mod extraction;
pub use extraction::{
    OffsetMap, RCodeChunk, VirtualSource, build_virtual_r_source, extract_inline_r_code,
    extract_r_chunks,
};
