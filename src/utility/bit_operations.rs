// A bit "mask" is a sequence of bits used to separate other bits from one another
// It's called a mask from the term masking tape. mask over that which you don't want changed.
pub fn mask(maskee: u32, masker: u32) {
    maskee & masker
}