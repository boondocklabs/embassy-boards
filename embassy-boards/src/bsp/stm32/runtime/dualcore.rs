use core::ptr::NonNull;

use crate::{
    BoardConfig,
    bsp::{BSP, stm32::shared_queue::SharedQueue},
    memory::BoardMemory,
};

/// Inter-core mailbox
pub struct Mailbox<M: 'static, const N: usize> {
    /// CM7 producer, CM4 consumer
    pub(crate) cm7_to_cm4: NonNull<SharedQueue<M, N>>,
    /// CM4 producer, CM7 consumer
    pub(crate) cm4_to_cm7: NonNull<SharedQueue<M, N>>,
}

impl<M: 'static, const N: usize> Mailbox<M, N> {
    pub fn new(init: bool) -> Self {
        // Configure addresses from the sections defined in the BSP memory layout
        let cm7_to_cm4 = <BSP<M> as BoardConfig>::Layout::MEMORY
            .section("SRAM4", "cm7_to_cm4")
            .expect("cm7_to_cm4 memory section in layout");
        let cm4_to_cm7 = <BSP<M> as BoardConfig>::Layout::MEMORY
            .section("SRAM4", "cm4_to_cm7")
            .expect("cm4_to_cm7 memory section in layout");

        // Check the sections are large enough
        let need = size_of::<SharedQueue<M, N>>();
        assert!(
            cm7_to_cm4.length >= need,
            "CM7 to CM4 memory section too small"
        );
        assert!(
            cm4_to_cm7.length >= need,
            "CM4 to CM7 memory section too small"
        );

        let mailbox = Self {
            cm7_to_cm4: NonNull::new(cm7_to_cm4.origin as *mut SharedQueue<M, N>)
                .expect("cm7_to_cm4 origin must not be null"),
            cm4_to_cm7: NonNull::new(cm4_to_cm7.origin as *mut SharedQueue<M, N>)
                .expect("cm4_to_cm7 origin must not be null"),
        };

        if init {
            // Initialize the shared queues
            unsafe {
                mailbox.cm7_to_cm4.write(SharedQueue::new());
                mailbox.cm4_to_cm7.write(SharedQueue::new());
            }
        }

        mailbox
    }
}
