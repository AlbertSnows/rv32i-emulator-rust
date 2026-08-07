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