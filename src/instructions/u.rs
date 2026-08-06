// U-type
//
//  31                                12 11     7 6      0
// |            imm[31:12]             |   rd    | opcode |
// |                20                 |    5    |   7    |
//
// lui:   rd <- imm << 12
// auipc: rd <- pc + (imm << 12)
// no register operands in, one register operand out, one 20-bit immediate
// that becomes the upper bits of a 32-bit value. used (with an I-type addi)
// to build large constants two instructions at a time.
// e.g. lui, auipc
