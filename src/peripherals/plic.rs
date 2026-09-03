// ## PLIC — Platform-Level Interrupt Controller
// PLIC is a coordinator between devices and our cpu software


// concepts
/// source_id: represents some sort of external device we acknowledge
/// hart: hardware thread, aka core
//// "the hart" = "the cpu
/// traps:  the cpu stops running normal instructions and jumps to run different code
/// mip: refer to addresses.rs, it's about what's currently interrupting/what's pending
/// trap handler: the code, whereever it is en memory, that handles the trap
/// context: a listener
//// a listener is a destination that con receive interrupt notifications
//// we have two: the CPU in either M mode or S mode
pub const NUM_SOURCES: usize = 96;
pub const NUM_CONTEXTS: usize = 2; // = 2 x # of harts
pub const M_CONTEXT: usize = 0;
pub const S_CONTEXT: usize = 1;


#[derive(Debug, PartialEq, Clone)]
pub struct PlicState {
    // represents: for source id I, how important is it? e.g. priority[I] = 3, bigger = more important
    pub(crate) priority: [u32; NUM_SOURCES + 1],
    // represents: is here a request waiting right now?
    pub(crate) pending: [bool; NUM_SOURCES + 1],
    // represents: for a given context C, C[source_id] represents "do i listen to interrupts for this source?"
    pub(crate) enabled: [[bool; NUM_SOURCES + 1]; NUM_CONTEXTS],
    // represents: how important does an interrupt have to be before i handle it?
    pub(crate) threshold: [u32; NUM_CONTEXTS],
    // armed[source_id] represents: is source_id currently allowed to create new requests?
    pub armed: [bool; NUM_SOURCES + 1],
}


const ENABLE_REGION_END: u32 = 0x2000 + 0x80 * NUM_CONTEXTS as u32 - 1;
const CONTEXT_REGION_END: u32 = 0x20_0000 + 0x1000 * NUM_CONTEXTS as u32 - 1;

impl PlicState {

    pub fn read(&mut self, offset: u32, _num_bytes: usize) -> u32 {
        match offset {
            0x000..0xFFC => {
                let word_index = offset / 4;
                self.priority[word_index as usize]
            },
            0x1000..0x107C => {
                let word_index = (offset - 0x1000) / 4;
                let mut result: u32 = 0;
                for i in 0..32 {
                    let source_id = 32 * word_index as usize + i;
                    if source_id <= NUM_SOURCES && self.pending[source_id] {
                        result |= 1 << i;
                    }
                }
                result
            },
            0x2000..=ENABLE_REGION_END => {
                let context = ((offset - 0x2000) / 0x80) as usize;
                let word_index = (offset - 0x2000 - (0x80 * context as u32)) / 4;
                let mut result: u32 = 0;
                for i in 0..32 {
                    let source_id = 32 * word_index as usize + i;
                    if source_id <= NUM_SOURCES && self.enabled[context][source_id] {
                        result |= 1 << i;
                    }
                }
                result
            },
            0x20_0000..=CONTEXT_REGION_END => {
                let context = ((offset - 0x20_000) / 0x1000) as usize;
                let local = (offset - 0x20_0000) % 0x1000;
                if (local == 0) {
                    self.threshold[context]
                } else if (local == 4) {
                    self.claim(context as usize)
                } else {
                    0
                }
            },
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u32, bytes: &[u8]) {
        let value = u32::from_le_bytes(bytes.try_into().unwrap());
        match offset {
            0x000..0xFFC => {
                let word_index = offset / 4;
                self.priority[word_index as usize] = value;
            },
            0x1000..0x107C => {}, // pending is read only
            0x2000..=ENABLE_REGION_END => {
                let context = ((offset - 0x2000) / 0x80) as usize;
                let word_index = (offset - 0x2000 - 0x80 * context as u32) / 4;
                for i in 0..32 {
                    let source_id = 32 * word_index as usize + i;
                    if source_id <= NUM_SOURCES {
                        self.enabled[context][source_id] = (value >> i) & 1 == 1;
                    }
                }

            },
            0x20_0000..=CONTEXT_REGION_END => {
                let context = ((offset - 0x20_000) / 0x1000) as usize;
                let local = (offset - 0x20_0000) % 0x1000;
                if (local == 0) {
                    self.threshold[context] = value;
                } else if (local == 4) {
                    self.complete(context, value)
                }
            },
            _ => {},
        };

    }

    // claim is responsible for selecting  whatever is the highest priority interrupt
    pub fn claim(&mut self, context: usize) -> u32 {
        let mut best = 0;
        for source_id in 1..=NUM_SOURCES {
            let is_candidate = self.pending[source_id] && self.enabled[context][source_id];
            if is_candidate && (best == 0 || self.priority[source_id] > self.priority[best]) {
                best = source_id;
            }
        }
        if best == 0 {
            return 0;
        }
        self.pending[best] = false;
        best as u32
    }

    pub fn complete(&mut self, context: usize, source_id: u32) {
        let is_subbed = self.enabled[context][source_id as usize];
        if is_subbed {
            self.armed[source_id as usize] = true;
        }
    }

    // answers the question:
    // is there any soruce id where
    // - this device has something waiting
    // - the listener is subscribed to it
    // - it's important enough to do now
    // compute_eip(context) asks the above question for *any* device for the given context
    pub fn compute_eip(&self, context: usize) -> bool {
        // Say priority[UART=10] = 3, priority[disk=2] = 5.
        // Say M-mode's threshold[0] = 4, S-mode's threshold[1] = 0.
        // Both devices pending and enabled for both.
        //
        //  compute_eip(M-mode): UART's 3 > 4? No. Disk's 5 > 4? Yes → EIP true for M-mode
        //      — but only because of the disk.
        //      If the disk weren't pending, M-mode's threshold of 4 would completely block the
        //      UART's 3 from ever waking it up at all.
        //  compute_eip(S-mode): UART's 3 > 0? Yes → EIP true for S-mode already,
        //      before even checking the disk. S-mode's threshold of 0 lets anything nonzero through.
        //  If M-mode does claim (say, the disk woke it): the comparison is priority[UART]=3 vs
        // priority[disk]=5 — disk wins, gets returned. Threshold never enters that comparison at all.

        (1..=NUM_SOURCES).any(|source_id| {
           self.pending[source_id]
                && self.enabled[context][source_id]
                && self.priority[source_id] > self.threshold[context]
        })
    }

    //
    pub fn set_pending(&mut self, source_id: usize) {
        let already_handling = !self.armed[source_id];
        if !already_handling {
            self.pending[source_id] = true;
            self.armed[source_id] = false;
        }
    }
}
