# Review

This document is meant to act as a delibrate review of the 
material of this project. The intent is to verify understanding
of design choices and motivations. 

## The question

How does a CPU work? When I click the power button, what happens
in the metal box sitting by my desk? How does a BIOS boot, and
how does an OS boot? This project explores answering that 
question.

## The CPU

A cpu is the brain of a computer. A computer, fundamentally,
takes a sequence of instructions and processes in a mutating
loop of steps repeatedly until and end state is reached or a
infinite subloop is executed forever. There's something about
the halting problem in there. 

Given that, we should expect a cpu to look something like this

```rust
let mut cpu = build_cpu();
let mut should_halt = false;
while !should_halt {
    step(cpu)
}
```

The entire complexity of a cpu lies in what it holds, and what 
step does to modify it. That will be part of the next 
investigation.

## An instruction: Add

Let's start with a goal; an instruction. The most basic yet useful
problem I would want a cpu to solve is adding two numbers. There
exists a specification called RISCV that outlines the necessary
compliance designs required to build what we commonly understand
to be a CPU. That includes all the necessary instructions needed
for a cpu to perform all the typical instructions we expect from
it. 

Let's revisit adding, how are we to add in RISCV? What do we need?
Let's look at the documentation first. The following is copied from 
the book.

---
### 2.4.2. Integer Register-Register Instructions

RV32I defines several arithmetic R-type operations. All operations
read the `rs1` and `rs2` registers as source operands and write the
result into register `rd`. The `funct7` and `funct3` fields select the
type of operation.

| `funct7`  | `rs2` | `rs1` | `funct3` | `rd` | `opcode` |
|-----------|-------|-------|----------|------|----------|
| bits 31:25 (7) | bits 24:20 (5) | bits 19:15 (5) | bits 14:12 (3) | bits 11:7 (5) | bits 6:0 (7) |

| Instruction | `funct7`  | `funct3` |
|-------------|-----------|----------|
| `ADD`       | `0000000` | `000`    |
| `SUB`       | `0100000` | `000`    |
| `SLL`       | `0000000` | `001`    |
| `SLT`       | `0000000` | `010`    |
| `SLTU`      | `0000000` | `011`    |
| `XOR`       | `0000000` | `100`    |
| `SRL`       | `0000000` | `101`    |
| `SRA`       | `0100000` | `101`    |
| `OR`        | `0000000` | `110`    |
| `AND`       | `0000000` | `111`    |

`ADD` performs the addition of `rs1` and `rs2`. `SUB` performs the
subtraction of `rs2` from `rs1`. Overflows are ignored and the low
`XLEN` bits of results are written to the destination `rd`. `SLT` and
`SLTU` perform signed and unsigned compares respectively, writing 1 to
`rd` if `rs1 < rs2`, 0 otherwise. Note, `SLTU rd, x0, rs2` sets `rd` to
1 if `rs2` is not equal to zero, otherwise sets `rd` to zero (assembler
pseudoinstruction `SNEZ rd, rs`). `AND`, `OR`, and `XOR` perform
bitwise logical operations.

`SLL`, `SRL`, and `SRA` perform logical left, logical right, and
arithmetic right shifts on the value in register `rs1` by the shift
amount held in the lower 5 bits of register `rs2`.

----
We know a computer operates on 1's and 0's. So this makes some amonut
of sense, but there's a bunch of terinology to explore here. Let's
highlight a set of tasks we need to address to try to get this 
instruction implemented as defined in the book. 

- what's an R type operation?
-  what's rs1, rs2, and rd?
-  what's a register? 
- what's a source operand 
- what's funct 7 and 3?
  - note, it says they select the type of operation, what does 
that mean? 
- what's an overflow? what does it mean to ignore one?
- what's XLEN?

The other parts talk about the other operations. We should
answer each question, and that should get us a solid head start
toward designing a CPU. 

Let's conclude with what we know so far, before answering each
question. 

- instructions are some sort of operation that do some...thing
- there are different types of instructions, add is R type
- an instruction seems to have several different "identifiers"
  - these identifiers discern both the kind of instruction
as well as the data needed to perform the operation
- these identifiers are encoded in bit ranges (Y:X) in...
something
  - QUESTION: what are the instructions encoded in?
- Add is identified by funct 7 (0000000) and funct3 (000)
  - it adds rs2, rs1, and puts the result in rd
  - Question: what does it mean to put these in rd? 

## Answering Questions: Add

By answering these questions, we should have, hopefully, the 
minimum needed to add two numbers. Remember, a program consists
of a state and a loop. Our cpu will process inputs, and 
presumably write the output somewhere. This brings up more
questions like, "where is rd, exactly?" as in, where do we actually
store the result of adding two numbers? Let's answer the questions
from before, and once we've done that, we should have enough
info to actually add two numbers. We'll implement it in the next
section. 

PLEASE NOTE: Going forward, I will cite the book instead of 
extracting quotes into here.

### what's an R type operation?

Refer to section 2.2. There are 4 core instruction formats, R/I/S/U. 
All are 32bits in length. Base ISA is IALIGN=32. Instructions must be 
aligned on a four-byte boundary in memory. We should throw an exception on
misalignment. riscv keeps rs1, rs2, and rd the same position in ALL formats
to simplify decoding, that's useful. Except 5-bit immediates in CSR,
immediates are always sign-extended, and are packed towards leftmost available
bits in the instruction. The sign bit is always bit 31. 

This poses more questions, to be appended onto our ever-growing list. 
- what is an immediate?
- what (in rust), represents 32 bits in length? 
- What does in mean to align on a four byte boundary? 
- what does it mean to be sign extended? 

All these questions will be answered as we progress. For now, the insight is that
for a given format F, all instructions under F have a specific kind of 32bit 
encoding.


###  What's rs1, rs2, and rd?
rs = reg source
rd = reg destination

What's a register? That's the next question! 


### What's a register?

This is not well-defined in the book. You could probably infer it, but I'd rather
look it up. You'll see online that it's a "storage" location *inside* the cpu
that's meant to be quickly accessible. Ok, so then rs1 is the register we get
the first value from, and second for rs2. Then, rd is the storage for where we
put the outcome. But how, exactly, do we "store" something in a register? 


### How do we store something in a register? 

Recall earlier, rd is rs1 + rs2. How do we represent that concretely? With a 
number. Say, 3 + 1. How is that represented in rust, on a computer? 
With bits. Remember everything amounts to bits, and CPUs involve a lot of 
interactions with bits. 

In rust, we have a type known as "u32". This is an "unsigned integer type."
So there's no sign. If there was a sign, that'd be i32. 
but for now, we'd have 3u32 + 1u32 = 432. What does that look like in bits?
That'd be 0b11 + 0b1 = 0b100. 

4 in binary is, of course, 0b100. So now we know how to sum two numbers in binary.
How do we store, say, 0b100 (or 4) in rd? What does a register look like? 
Well, on a physical CPU it's literal silicone on a chip somewhere. We 
don't have that, we just want to *emulate* a cpu. AKA, given some inputs
we sohuld be able to recreate the outputs reliably and consistently. 
How do we do that? Well, we should think about what sort of software
we'd want to use to represent a register. How should we represent a register?
Well, we're storing binary. How best would we store binary? 

I don't think there's a super obvious answer to this in the book, but recall
that in the format section it said
> There are 4 core instruction formats, R/I/S/U.
All are 32bits in length. Base ISA is IALIGN=32. Instructions must be
aligned on a four-byte boundary in memory.

So all instructions are 32 bits in length. When we write instructions out,
they're gonna be 32bit. Should rd match our instructions? The answer 
becomes clear with a question that hasn't been answered yet; XLEN. 


### What's XLEN? 

Section 1.3 of the book, quote: 

> We use the term XLEN to refer to the width of an integer register 
in bits (32 or 64)

There is a useful answer. 4 is an integer, and we're gonna make a 32bit
cpu in this project. Thus, we store integers in a 32bit register. Also
recall that our instructions are also stored as u32bits. That means
when we read instructions, they have the same shape. At this point, 
it seems like sign are pointing us to using 32bits as the size for our 
registers. Let's keep that in mind, and move on. 



### what's a source operand?

Take 3 + 1 = 4. 3 and 1 are operands. 
A source operand is, well, the source of an operand. 
Where do we get 3 from? rs1, so rs1 is the source operand. 
Another way to think of it, it's the address where 3 lives. 

### what's funct 7 and 3?

these seems to just be identifiers. A permutation of 7 and 3 defines
the kind of instruction. E.g. add vs addi

So if we read an instruction's 7 and 3, that should tell us if we're 
adding or doing a different operation. 




### what's an overflow? what does it mean to ignore one?

In math, numbers don't overflow. In computers, they do.
You cannot store infinity in a rock, so our machines have a finite limit
on number size. In u32, that's u32::MAX in rust. 
What happens when we do u32::MAX + 1? one of two things:
1. if we just add, the computer crashes because it cannot add beyond that.
2. it overflows

To overflow is just what happens in binary. 

u32::MAX = 1111_1111_1111_1111_1111_1111_1111_1111
or in hex: 0xFFFFFFFF

Imagine 3 + 1 again in binary, what happens? 

0b11 + 0b1 = 0b100

The bits reset, and the next bit is flipped on. But what happens when
there's no next bit to flip?

Then it just...resets. Imagine u32::MAX = 0b11
If you do 0b11 + 1, now you'd get 0b00 instead. That's what it
means to overflow. So we ignore any concerns or considerations regarding
adding too large numbers, and instead just reset to 0.


### how are instructions encoded?

Instructions are encoded as 32bit numbers, as explained earlier and in
the book. In fact, let's actually look at how add would be encoded per
the spec. Instructions are about registers, not acutal number. Recall 
the question about "source operand" above. So instead of saying 
3 + 1, we say "add the numbers at rs1 and rs2"
How do we encode that? 

Recall the 32bit definition for ADD we looked at earlier.


| `funct7`  | `rs2` | `rs1` | `funct3` | `rd` | `opcode` |
|-----------|-------|-------|----------|------|----------|
| bits 31:25 (7) | bits 24:20 (5) | bits 19:15 (5) | bits 14:12 (3) | bits 11:7 (5) | bits 6:0 (7) |

| Instruction | `funct7`  | `funct3` |
|-------------|-----------|----------|
| `ADD`       | `0000000` | `000`    |

Also remember that the book cares about instructions being
divisible by 4. For readability, I will be breaking them into
groups of four, aka a byte. Where do we want to get our numbers 
from? rs1, and rs2? Let's just say addresses 1 and 2. Where
should we store our address? Let's store it in 3. 


Then we have,
- funct 7 = 0000_000
- funct 3 = 000
- rs1 = 1 (or 0x1 in hex)
- rs2 = 2
- rd = 3
- op code = ?

What's op code? We'll come to that in the next question. For now
let's see what the bit encoding would look like, minus op code.

[0000_000][0_0010]_[0000_1][000]_[0001_1][???_????]

the [] signify a chunk of the encoding, but raw it would look 
something like this:
0000000000100000100000011???????

That's it, the whole instruction. This does make sense, since we
know that ultimately everything decomposes to a 0 or 1 in 
computers. But what's op code? 


### What's an op code? 

The op code just signifies which format type you're working in,
e.g. R or S. It's 7 bits long.

If you refer to table 72, you can reverse engineer what the op 
code should be, given the context is ADD, which falls under OP.
That amounts to 01100??, and we see inst[1:0] = 11 so we get
0110011

Thus, the final encoded instruction for adding two numbers
in the most basic way possible is:

00000000001000001000000110110011 

or (split into groups of 8)

00000000_00100000_10000001_10110011

So now we know how to both code, and uncode, instructions.
This is an excellent state to be in. Let's continue.

### what is an immediate?

In Add, we take rs1 + rs2. Sometimes, instead of saying "go here
to add this number" we instead already have the acutal number.
So instead of rs1 + rs2, we have rs1 + the immediate value we 
want to add, aka imm.

Thus, imm is a bit encoding of the immediate value we want to
add, in the instruction. So if we wanted to do 
rs1 + imm, which may be 3 + 7, the 7 would be encoded in the u32
in 0b0111


### what (in rust), represents 32 bits in length?

u32 in rust, an unsigned 32 bit integer

### What does in mean to align on a four byte boundary?

This one has to do with memory. Recall that we intend to go
forward storing things in a 32bit format. 32bit is 4 bytes
(or 4 x 8 bits).

If we go forward assuming we're storing 32bit numbers, then
it's safe to assume we shouldn't randomly find ourselves looking
at address 6, for example, since that's halfway through a 32bit
number. 

By the way, a Word is a 32bit number, we'll use that going
forward. 64bits is double word.

### what does it mean to be sign extended?

Signed bits are indicated by 1 or 0 at the end of a number.

0b1101 is -3 as a u4 bit. 