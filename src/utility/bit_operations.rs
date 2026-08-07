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