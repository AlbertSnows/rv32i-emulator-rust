use crate::definitions::cpu_definition::MemoryState;

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

    #[test]
    fn test_mask() {
        // add x3, x1, x2 -- rd=3, sitting at bits 11-7, unshifted.
        let raw_word = 0x002081B3;
        let result = mask(raw_word, masks::REG_DESTINATION);
        assert_eq!(result, 0b0001_1000_0000); // 3, still positioned at bits 11-7
    }

    #[test]
    fn test_mask_and_shift() {
        // same word, but now every field comes back shifted down to bit 0.
        let raw_word = 0x002081B3;
        assert_eq!(mask_and_shift(raw_word, masks::REG_DESTINATION), 3);
        assert_eq!(mask_and_shift(raw_word, masks::FUNCT_3), 0);
        assert_eq!(mask_and_shift(raw_word, masks::REG_SOURCE_ONE), 1);
        assert_eq!(mask_and_shift(raw_word, masks::REG_SOURCE_TWO), 2);
        assert_eq!(mask_and_shift(raw_word, masks::FUNCT_7), 0);
    }
}