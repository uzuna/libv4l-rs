use std::alloc::{alloc, Layout};

/// get aligned memory block
///
/// Reference from: https://qiita.com/moriai/items/67761b3c0d83da3b6bb5
pub fn aligned_alloc(size: usize, align: usize) -> Vec<u8> {
    unsafe {
        let layout = Layout::from_size_align(size, align).unwrap();
        let raw_mem = alloc(layout);
        Vec::from_raw_parts(raw_mem, size, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // check the boundary of the memory block
    fn bound(mem: &[u8]) -> usize {
        let addr = (&mem[0] as *const u8) as u64;
        let mut bound: u64 = 1;
        while addr & bound == 0 {
            bound <<= 1;
        }
        bound as usize
    }

    // boundary is equal to or greater than align
    #[test]
    fn test_aligned_alloc() {
        let td = vec![16, 32, 64, 128, 1024, 4096, 8192];
        for &align in &td {
            for _ in 0..16 {
                let size = 1024;
                let buf = aligned_alloc(size, align);
                assert!(bound(&buf) >= align);
            }
        }
    }
}
