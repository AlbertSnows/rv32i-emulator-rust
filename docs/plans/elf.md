# ELF symbol lookup notes (finding `tohost`)

ELF header is bytes 0-51:

- `shoff` -> byte 32
- `shentsize` -> byte 46
- `shnum` -> byte 48

## Where do we find `tohost`?

`shoff` points to -> section header table.

Section header table:

- `shnum` entries
- `shentsize` bytes each
- each entry has an `sh_type`
- we care about `sh_type = 2`, `sh_type = 3`

Type 2 has rows with:

- (?) `st_name`, `st_value`

Type 3 has a bunch of bytes. One will have, at location X, `tohost`.
`tohost` maps to `st_name` from type 2.

From the mapping, we know `st_value`, which has our address.

### Loop 1

- walk `shnum`
- look at `sh_type`
- find `sh_type = 2`
- `sh_link` points to `.strtab`

### Loop 2

- walk records inside symbol table (what is symbol table?)
- resolve `st_name` for record
- if `st_name = tohost`, yield `st_value`
