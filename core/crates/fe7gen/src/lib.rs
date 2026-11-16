//! Recompiled game code.
//!
//! The repository ships no game code. `tools/build.sh <your-rom.gba>` translates
//! the code of YOUR OWN ROM into Rust files in `core/generated/` (ignored by git,
//! never publish them) and builds with the `rom` feature. Without it everything
//! runs on the interpreter, which plays exactly the same game, just slower.

#[cfg(feature = "rom")]
#[path = "../../../generated/lib.rs"]
mod rom;
#[cfg(feature = "rom")]
pub use rom::*;

#[cfg(not(feature = "rom"))]
mod stub {
    use gba::BlockFn;
    pub const BLOCK_COUNT: usize = 0;
    pub static BLOCKS: &[(u32, bool, BlockFn, u32)] = &[];
    pub fn lookup(_addr: u32, _thumb: bool) -> Option<BlockFn> {
        None
    }
    pub fn lookup_verify(_addr: u32, _thumb: bool) -> Option<(BlockFn, u32)> {
        None
    }
    pub fn lookup_index(_addr: u32, _thumb: bool) -> Option<usize> {
        None
    }
}
#[cfg(not(feature = "rom"))]
pub use stub::*;
