//! Prepared runtime identity retained as a WASM custom section for offline export.
#[cfg(all(feature = "fonts", feature = "physics"))]
const INFO: &str = concat!("1\n", env!("CARGO_PKG_VERSION"), "\nfonts=1\nphysics=1\n");
#[cfg(all(feature = "fonts", not(feature = "physics")))]
const INFO: &str = concat!("1\n", env!("CARGO_PKG_VERSION"), "\nfonts=1\nphysics=0\n");
#[cfg(all(not(feature = "fonts"), feature = "physics"))]
const INFO: &str = concat!("1\n", env!("CARGO_PKG_VERSION"), "\nfonts=0\nphysics=1\n");
#[cfg(all(not(feature = "fonts"), not(feature = "physics")))]
const INFO: &str = concat!("1\n", env!("CARGO_PKG_VERSION"), "\nfonts=0\nphysics=0\n");
const fn bytes<const N: usize>(text: &str) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = text.as_bytes()[i];
        i += 1;
    }
    out
}
#[used]
#[link_section = "kiln.runtime.v1"]
static METADATA: [u8; INFO.len()] = bytes(INFO);
