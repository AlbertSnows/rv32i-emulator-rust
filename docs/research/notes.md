3 bit thing in the middle, what is it?

it's funct 3. it helps the op code id the specific instruction.


what about the remaining 7 bits for slli, srli, and srai?
srli differs from srai via funct7, the remaining 7 bits? 
addi through andi use up all of func3, so the remaining 7 bits

----
shamt has 5 bits
imm has 12
why imm have 12?
imm has 12 because that's the remainder, there's no special reason. 
shamt has 5 because that gives the range 0-31 to shift a 32 bit value, which is all it needs.

-------------------------------------------
old notes
-------------------------------------------
machine language

assembly language


digital computer (there's also analog, quantum)

volatile = forgets without power

registers are small data blocks on the cpu. 

main memory - connected to cpu, holds data not able to fit on the cpu

register has a number. 

main memory is broken up into blocks, called bytes.

bytes are assigned a number called an address, aka its location in memory

non-volatile, stored between power

execution unit coordinates instruction operation

ALU = arithmetic and logical unit.

ALU calculates outcome of insturctions.
circut that doess tho computation. everything else is storage and routing. 
mechanically it's a combinational circuit. 

---
rv32 cpu
32 general purpose registers. each w/ 32 bits.
special purpose registers: x0, pc
x0 is always 0/logical false.
pc = program counter. cpu uses it to remember memory addresses where program instructions are located.

XLEN = width of an integer in bits (32, 64, 128)

ISA = catalog of rules. describes instructions and features for a cpu.

how is a program executed?
- instruction cycles
-- fetch -> decode -> execute

hart = hardware thread

assembly language components
- 1 line o text containing an instruction or directive
- instruction - label, mnemonic, operands, comment
- directive - control the operation of the assembler

memory layout

register file:
bank of fast storage. built directly into the core. 

machine state:
- register file
- program counter
- memory

jump = an unconditional branch

a latch is a collection of logic gates that can store memory.
a group of latches is a register
------
example from refrence sheet

add
usage: rd, rs1, rs2, 
type: R
details:
rd <- rs1 + rs2, pc <- pc+4
meaning:
rd = destination
given rs1 + rs2
at the same time
pc is set to pc+4

add immediate:
rd, rs1, imm, 
type = i
value is stored in instruction itself
rs1 + imm_i(immediate value)

what's auipc?
rd <- pc + immu_, pc <- pc + 4
it copies the pc into register (plus some value)

branches update pc conditionally, using imm_b if appropriate

jump = go somewhere, remember where i was

----
format types

each  format is shaped around what category of instructions needs an operand?

r type: two inputs, one output, no immediate.
i type - one input, one output, one constant. usses a 12 bit emmediate.
s type - two inputs, no output, one constant. destination is a memory address?
b type - same shape as s type. two i, no rd. immediate is a branch offset instead of memory. pc relative.
u type. one output, 20 bit constant, no inputs. for large constants, no need to read registers. 
j type. one output, one big constant, no inputs. same as u type. constant means jump target ofset, instead of raw upper bits.

important questions formats answer:
- how many registers does this instruction read
- does it write to a register
- how large of a constant does it need?

format is the shape of an interface to an instruction. 
format = "what are my inputs and outputs"
opcode = "what do i do with my inputs and outputs"

format is for an instruction set. instruction set is 4 bytes, 32 bits.

endianness refresher:
when a value is wider than one byte, which byte goes at the lower memory address? 
- little: least significant byte at lowest address
- big: most significant byte first

example: 
suppose the 32 bit value 0x12345678 is stored at 0x1000
- little endian: 0x1000=0x78, 0x1001=0x56, 0x1002=0x34, 0x1003=0x12
- big endian: 0x1000=0x12, 0x1001=0x34, 0x1002=0x56, 0x1003=0x78

structure
- register file: 32 slots array?
- PC: one number, keeps track of current instruction
- memory: big byte array
----------------
what is an immediate? 
- constant value
- baked directly into the instruction's own bits
- no lookup needed

what is op code?
- field, tells decode what kind of instruction it is
- 6 formats, 7 bits, 