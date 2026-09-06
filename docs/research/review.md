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


## Summarizing our notes

We've learned a lot. 

We know this, we essentially want to take a number like this:

00000000_00100000_10000001_10110011

and decompose that into an ADD instruction to pass into the cpu. 

...How do we do that? 

Well, we know the input, how do we get the output? Let's thing about what
we need. 

We need
- something to "decode" this instruction into its constituent parts
  - and something to store it in, call this format storage (FS)
- given FS, we need to map that to an adding function
  - in the case that the identifiers map to "add"
- once we've decoded the instruction and know what type of instruction it
is, we need to perform that operation and store it somewhere
  - our cpu is our storage for our registers, so we need register storage
in our cpu


That's enough to work with. This repo is in a completed state, so there is
an immense amount of complexity. We will handwave that complexity and
ignore it until it becomes relevant with time. 


Let's start with the decoder. 


## The Decoder


Refer to `decode_word_to_instruction`.

In the case of a half word, 16 bits, we don't have
the same op code we normally have, so that gets
special handling. For the rest,
we have an op code for each kind of format
or even sub formats. Given the op code,
we know the format of the 32 bits, and
all we need to do is parse those bits
in the appropriate way as outlined
in the specification. 

If the opcode is unrecognized,
we can't handle it. We throw a trap cause, which
we will revisit later. 


## Parsing


Parsing an instruction introduces several concepts to go over. 
- mask and shift
- shake to signed
- conform to format type
- merge bits

For conforming, we define a "Format" type which contains all the 
information an instruction needs to perform its associated operation. 

These concepts become relevant for other instructions, but 
let's go ahead and look at these unstructions.


### Mask and Shift

We do two things, masking and shifting. 

#### Masking

Masking is just a bit operation. X & Y, such that 1 & 1 = 1, anything else = 0

so take 0b101 & 0b001, that gives us 0b001 since both only have 1 in the 0th position.

### Shifting

Shifting is how we move bits up and down in position in a bit. 


If I do 1 << 3, I get 0b1000, which equals 8
likewise, 8 >> 3 = 1.

Note, it shifts the whole string, so 0b101 << 2 = 0b10100, and 0b101 >> 1 = 0b10.


So making and shifting takes a number and only keeps a specific part of it.
After that, it looks at how many 0's are at the start (the right side), and removes them.


so 0b10100 becomes 0b101

### Shake to Signed

Supposed we have 0b101, that's either 5 or -3 depending on whether it's 
signed or unsigned. Assume signed. That's a u3 type. To do this correctly, 
we need to 1 extend instead of 0 extend. 

So 0b101 becomes 0b1111_1101, which is in fact -3. 

So shake to signed moves the current bit up 32 bits and back down to 
correctly convert it to signed. So we have 0b101 as u3, we want u8. 

We then have 0b0000_0101, and we "shake" the "101" part up to the top,
then back down, so it goes to 0b1010_0000 then to 0b1111_1101

### Merge Bits

just take a list of (bit, location) and move that bit into its
corresponding location in the new constructed bit integer. 

## Executing an instruction

This may seem a bit early. How do we execute an instruction we can't
store? The latter is not required for the former. Let's think about
how we should organize mapping these formats to their corresponding 
instructions. Then, we can come back to ADD, and think about what's left.

Each cycle of a cpu executes an instruction. I call a cycle a step, so 
let's refer to step. step has a bunch of contextual checks, and then an actual 
"perform_step" function. Here, we see the core structure.

1. fetch from memory (to be revisited shortly)
2. decode 
3. execute instruction
4. advance pc

We could just pass in an instruction to our loop as an arg and call it a day. 
The ultimate goal of this project is to simulate a basic OS. To do that, we need
memory. Memory is "longer" term storage, but is also slower to access mechanically. 

So instead when we do finally run an add instruction, we should try to read it from 
memory somewhere. We will revisit this later. Instead, let's worry about executing
the instruction. Then, we'll worry about the last step, advancing the cpu's state. 

Each execution has its own arm, in each arm, we further parse out the instruction
from the format. For example, give our R op code from earlier, we use funct 7 and
funct 3 to identify that our R instruction is "ADD" and execute accordingly.

Now we finally reach our next big impasse. Say we want to add 3 and 1, where are they 
stored. And where do we store 4? 

## Storing registers

Recall our definition of ADD, rd1 + rd2 = rd

AKA, we need to read from rs1, rs2 and store the result in rd. HOW???

This is what we emulate. 1, 3, and 4 are a number; a u32. We need to store it
in our register. We know a register is 32 bits. How many registers do we have? 
Refer to table 2 in section 2.1. We have 32 registers + pc, starting at 0.
We will revisit pc.

And with that we have our core notion of a cpu. It's essentially a 32x32bit array. 
Now, we can finally define our CPUState. 

It starts with our register file, a 32x32 bit array. We define a way to read and write.
From the docs, register spot 0 can never be written to, so when we write, if the
location is 0, it's a no op. 

By the way, wrapping_add is how you overflow in rust. 

And that's it. we take x1, x2, store in x3...now what?

## PC

What do we do after we've completed an instruction? Generally, a cpu performs an arbitrary number
of steps before halting. How do we get to the next instruction? With PC. PC stands for 
program counter. Its only job is to hold the address of the next instruction to perform.

For example, suppose we want to add x1 + x2 and store in x3. 

Remember, our instruction is:
00000000_00100000_10000001_10110011

Let's call this instruction I_ADD. 

where do we store this? In memory! but to execute an instruction we need to
point our PC to it. For now, let's store I_ADD at location 8 in memory. Then, what happens if we
set PC = 8? It reads 8, decodes it, executes it, and by the end reg_file[3] should be 4! 

But then what? 

The specification has a definition for each pc increment. For add, we take pc to +4
Remember a word is 4 bytes, so for a 32bit cpu, we walk through memory one Word at a time,
depending on the instruction. Each instruction has its own specifications for what happens
to pc after it finishes. 

Now we have two things to worry about
- how do we define memory?
- how do we increment pc? 

## Memory

Memory is just like a reg file, but stored elsewhere. You typically need a lot more of it
to store all the instructions you want to run. To boot a basic OS, you need a magnitude in 
the MiBytes of size to perform. It has no overflow, so if size = 10, and you write to 11, throw
an exception. Same for reading. The Memory is OUTSIDE the CPU. For a cpu to talk to something 
outside its own boundaries, we use a BUS. We wil revist busses shortly.


## Advance PC

The state of PC is well defined. After each execution, we advance it in accordance with the specs.
B Types are branching in nature, and thus directly modify PC.

For Jalr, refer to the spec:
> "Clearing the least-significant bit when calculating the JALR target address both simplifies 
> the hardware slightly and allows the low bit of function pointers to be used to store auxiliary 
> information."



## Bussin

A bus is borrowed from electrical engineering, a "busbar" is a shared metal rail that distributes 
power/signal to multiple points at once. In computing, it came to mean a shared set of wires connecting 
the CPU to memory and peripherals, so one address can reach any of several devices depending on 
which range it falls in.

This is a well-defined concept. The cpu has one job: perform an instruction, and write it out 
to some address. Suppose that address is 10, what does 10 mean? It doesn't matter. The bus's 
job is to redirect the address to the appropriate place, which removes responsibility from
the cpu to worry about things like memory and peripherals. 

For this project, there are several separate addresses the cpu might be concerned about. Normally,
these would be stored in memory, but in our case we can just represent the different components
with different structs. 

### Accessing

Looking at bus, you can see that *directly* writing operates over a bit range. 
Things like MTIME-MTIME_END, PLIC-PLIC_END, etc. 

These are identified by literal addresses in memory. Thus, we could in theory store them in
our ram, but instead we've opted to make them their own data structures. 

Notice we also have guest reading and writing, we will come back to that later. 

#### Access Types

How do you prevent a hacker from accessing your computer? A password, generally.

Likewise, a cpu has a concept of access types. Except, it's not to prevent "evil" attempts
to access the cpu. Instead, it's meant to control what types of instructions can change
memory. Imagine a user tries to change the instruction at location L, but L keeps the computer
from shutting off. If we give everyone equal access, your computer shuts down. If we say, 
"you can only touch L if you access level is, say, H". That's the idea behind access types,
which you will see throughout the project. 


# Check in: Post Bus

So now what do we know?
- we know how to decode an instruction
- we know how to compute the outcome of an instruction
- how to save it back to the register or memory

This is a critical point. Conceptually, we have the core flow of a cpu system outlined in 
its normal path. Going forward, everything else is expanding on the capabilities of our cpu
to perform various tasks that is needed to run a complex program. At this point, other questions
may come to mind. Namely,

- how do we handle when something goes wrong
- how do we add tests as we flesh out the spec?
- how do we add peripherals
- how do we boot into a full system? 
- and much more

These questions will be answered in time. Going forward, we're going to try going file-by-file
throughout the system, each directory at a time. If a non-obvious concept is assumed or introduced
that isn't cited, you should assume it was looked up/researched elsewhere online.

## Trapping

Trapping is when something goes "wrong". Except, that's actually to narrow. Trapping is a sort
of emergency handler situation. When a trap occurs, we are telling the cpu to do the following:
- stop executing instructions at the Current Location (CL)
- instead, look at the instruction at the Trap Location (TL)
- store CL at the Return Location (RL)
- proceed from TL until it instructs you to fetch CL from RL, and continue. 

So the idea is essentially to "jump" to a new location to handle an important case before continuing
the program it was running before. 

There is much to go over in the trapping section, outlined below.

### TODO: trapping 

## The CSR

The CSR is Control and Status Register. It is 12 bits wide, which means every address that
has to with the CSR fits in a 12-bit wide context.  This is a complex file doing many things
so I am going to go over each part in steps.

### Cycles

The cpu uses cycles to keep track of time, as well
as other concepts. Defined under the Zicntr extension.

#### CYCLE

> "a count of the number of clock cycles executed by the processor core... from an arbitrary start time in the past."

Used for performance monitoring

#### TIME

> "wall-clock real time that has passed from an arbitrary start time in the past."

Used for time reasoning purposes

#### INSTRET

> "Instructions that cause synchronous exceptions, including ECALL and EBREAK, are not considered to retire and hence do not increment the instret CSR."

A count of retired instructions. 

### Flags

Used to keep track of if we're skipping a retirement. 

### Registers

#### Trap handling, M Mode

- mstatus 
  - contains many bit definitions, see the bit field under addresses
- mtvec
  - the address an M-mode trap jumps to
- mepc
  - when the cpu traps, it stores its current PC here for when it returns
- mcause
  - an id of the specific trap that occurred
- mtval
  - more specific information about the trap case
- mscratch
  - scratch space, meant to temporarily store data for the handler

#### Trap handling, S Mode

- SSTATUS 
  - S mode is unable to view certain bits in the STATUS register, which
  - is what this represents
- STVEC, EPC, CAUSE, TVAL, and SCRATCH
  - mimic their M mode counterparts

#### Interrupting

- MIE - a bit field definition for which interrupts are enabled
(aka can be listened to, see interrupts for more details)
  - enabled is about "do I allow this to interrupt me"
- MIP - a bit field definition for which interrupts are pending, alongside MIP
- SIE/SIP - S mode masks over ie and ip
- MIDELEG - controls interrupts
- MEDELEG - controls exceptions
  - both legs, they're used to discern if the exception should be in M mode or
S mode

#### Counters

- H suffix
  - for all teh "H" suffix concepts, these need to be u64 bit counters (for 
example, the cycles). Since we use 32bit, the H registers are essentially the
extra 32 bits needed for these counters. The H stands for high, as in the high
half of the bits. 
- COUNTEREN - these gate access to the cycles, but aren't used

#### Virtual Memory

- SATP 
  - Supervisor Address Translation and Protection
  - this register activates virtual memory, and directs the CPU
to the page tables (refer to the virtual memory section)
    - contains MODE, ASID, and PPN
      - ASID: TODO
      - PPN: TODO

#### Identification
- MHARTID
  - a hart is a core. this tracks the different cores on a multi-core system
- MVENDORID, MARCHID, MIMPID
  - these are specific hardcoded values to identify hardware, we don't need them
- MISA
  - contains a definition of what features a hart uses. A/I/U, etc.

#### Debug/Trigger
- TSELECT - unused
- TDATA1, TDATA2 - unused
- TCONTROL - unused

#### Physical Memory Protection

- PMPCFG
  - unused
- PMPADDR
  - unused

#### MISC
- MCOUNTINHIBIT
  - not used
- MSTATUSH
  - the upper half of mstatus, gives more configuration options





todo:
- mmu
- interrupts
- bit field
- modes
- virt mem