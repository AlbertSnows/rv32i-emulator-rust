use crate::definitions::cpu_definition::MemoryState;

pub fn set_bit_range(original_bits: u32, new_bits: u32, width: usize, shift: usize) -> u32 {
    let bits_to_update = ((1u32 << width) -1 ) << shift; 
    let bits_to_preserve = original_bits & !bits_to_update;
    let positioned_new_bits = (new_bits << shift) & bits_to_update;
    bits_to_preserve | positioned_new_bits
}


// The width will be 31 - position + 1
pub fn shake_to_signed(unsigned_bits: u32, width: u32) -> i32 {
    let shift_distance = 32 - width;
    ((unsigned_bits << shift_distance) as i32) >> shift_distance
}

pub fn merge_bits(bit_list: &[(u32, u32)]) -> u32 {
    bit_list.iter().fold(0, |acc_bit, &(value, shift)|{
        acc_bit | (value << shift)
    })
}

pub fn store_in_mem(data: &[u8], mem: &mut MemoryState, location: usize) {
    assert!(location + data.len() <= mem.storage.len());
    let placement_range = location..(location+data.len());
    // copy from slice takes every element from a source slice and copies it
    // in order into the slice you call it on
    // example: 
    // mem.storage is  [0, 0, 0, 0, 0, 0, 0, 0] 
    // data is [0xB3, 0x81, 0x20, 0x00]
    // Call mem.storage[0..4].copy_from_slice(data)
    // mem.storage is now [0xB3, 0x81, 0x20, 0x00, 0, 0, 0, 0] 
    mem.storage[placement_range].copy_from_slice(data)
}

// A bit "mask" is a sequence of bits used to separate other bits from one another
// It's called a mask from the term masking tape. mask over that which you don't want changed.
pub fn mask(maskee: u32, masker: u32) -> u32 {
    maskee & masker
}

pub fn mask_and_shift(maskee: u32, masker: u32) -> u32 {
    let unmoved_output = mask(maskee, masker);
    let final_masking = unmoved_output >> masker.trailing_zeros();
    final_masking
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::masks;
    use crate::programs::instructions::ADD_X3_X1_X2;

    #[test]
    fn test_mask() {
        // add x3, x1, x2 -- rd=3, sitting at bits 11-7, unshifted.
        let raw_word = ADD_X3_X1_X2;
        let result = mask(raw_word, masks::REG_DESTINATION);
        assert_eq!(result, 0b0001_1000_0000); // 3, still positioned at bits 11-7
    }

    #[test]
    fn test_mask_and_shift() {
        // same word, but now every field comes back shifted down to bit 0.
        let raw_word = ADD_X3_X1_X2;
        assert_eq!(mask_and_shift(raw_word, masks::REG_DESTINATION), 3);
        assert_eq!(mask_and_shift(raw_word, masks::FUNCT_THREE), 0);
        assert_eq!(mask_and_shift(raw_word, masks::REG_SOURCE_ONE), 1);
        assert_eq!(mask_and_shift(raw_word, masks::REG_SOURCE_TWO), 2);
        assert_eq!(mask_and_shift(raw_word, masks::FUNCT_SEVEN), 0);
    }

    #[test]
    fn test_store_in_mem() {
        let mut mem = crate::definitions::cpu_definition::build_memory_state();
        store_in_mem(&[0xB3, 0x81, 0x20, 0x00], &mut mem, 0);
        assert_eq!(mem.storage[0..4], [0xB3, 0x81, 0x20, 0x00]);
    }

    #[test]
    fn test_store_in_mem_at_offset() {
        let mut mem = crate::definitions::cpu_definition::build_memory_state();
        store_in_mem(&[0xAA, 0xBB], &mut mem, 10);
        assert_eq!(mem.storage[10..12], [0xAA, 0xBB]);
        // untouched neighbors stay zero
        assert_eq!(mem.storage[9], 0);
        assert_eq!(mem.storage[12], 0);
    }

    #[test]
    fn test_merge_bits() {
        // 0b101 shifted up by 4 (0b1010000), OR'd with 0b11 sitting at the bottom
        let result = merge_bits(&[(0b101, 4), (0b11, 0)]);
        assert_eq!(result, 0b1010011);
    }

    #[test]
    fn test_merge_bits_empty_is_zero() {
        assert_eq!(merge_bits(&[]), 0);
    }

    #[test]
    fn test_set_bit_range() {
        // Worked by hand: mask = 0b011100; be & !mask = 0b00001;
        // (nb << 2) & mask = 0b00100; 0b00001 | 0b00100 = 0b00101.
        let be = 0b10001;
        let nb = 0b001;
        let width = 3;
        let shift = 2;
        assert_eq!(set_bit_range(be, nb, width, shift), 0b00101);
    }

    #[test]
    fn test_shake_to_signed_positive_stays_positive() {
        // 5 in a 13-bit field, sign bit (12) unset -- should read as plain 5
        assert_eq!(shake_to_signed(5, 13), 5);
    }

    #[test]
    fn test_shake_to_signed_negative() {
        // 8188 is -4's bit pattern in a 13-bit field (sign bit 12 set) --
        // same value worked through by hand for b.rs's imm reassembly.
        assert_eq!(shake_to_signed(8188, 13), -4);
    }
}