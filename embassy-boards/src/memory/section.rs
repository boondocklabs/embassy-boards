use super::MpuAttrs;

/// Resolved memory section
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemorySection {
    pub name: &'static str,
    pub origin: usize,
    pub length: usize,
    pub align: usize,
    pub mpu: Option<MpuAttrs>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemorySectionSpec {
    pub name: &'static str,
    pub offset: Option<usize>,
    pub length: usize,
    pub align: usize,
    pub mpu: Option<MpuAttrs>,
}

impl MemorySectionSpec {
    pub const fn new(name: &'static str, length: usize) -> Self {
        Self {
            name,
            offset: None,
            length,
            align: 4,
            mpu: None,
        }
    }

    pub const fn with_align(mut self, align: usize) -> Self {
        self.align = align;
        self
    }

    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub const fn with_mpu(mut self, attrs: MpuAttrs) -> Self {
        self.mpu = Some(attrs);
        self
    }
}
