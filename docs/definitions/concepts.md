# Ideas

## Complement

The complement of x with respect to b is b - x.


## One's Complement

Take 3

4-bit: 0011

Flip it: 1100 (12)



## Two's Complement

Two's complement means the complement relative to the power of 2 (2^n).

In binary, for negative numbers, the leftmost number is thought to be negative, and all remaining bits are positive.
The summation of -leftmost + (all remaining bits) will sum up to your number.

Suppose we have -4

4-bit: 1100 = -8 + 4 + 0 + 0
8-bit: 1111_1100 = -128+64+32+16+8+4
etc.

Another way to think of it, suppose we have 4.

4-bit: 0100

Question: what bit pattern represents its negative under two's compliment? 

Answer: 1011 + 1 = 1100 = -8 + 4 = 4

