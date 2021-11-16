Seidr bytecode is a binary representation of the abstract syntax tree.

The top level is a list of items.

# Items

Items is a u64 number of items followed by the items.

# Item

An item is an expression

# Value Expression

An expression has a tag and associated data

| type         | tag | data                            |
| ------------ | --- | ------------------------------- |
| number       | 0   | f64 value                       |
| char         | 1   | utf8-encoded character          |
| static array | 2   | u64 length, type tag, item data |
| unary        | 3   | function, value                 |
| binary       | 4   | function, value, value          |

# Function Expression

| type             | tag | data                                                  |
| ---------------- | --- | ----------------------------------------------------- |
| operator         | 16  | opcode                                                |
| function literal | 17  | items                                                 |
| unary            | 18  | unary modifier, function                              |
| binary           | 19  | binary modifier, value or function, value or function |
| atop             | 20  | function, function                                    |
| fork             | 21  | value or function, function, function                 |

## Opcodes

| name                   | opcode |
| ---------------------- | ------ |
| identity/right         | 0      |
| plus                   | 1      |
| negate/subtract        | 2      |
| sign/multiply          | 3      |
| reciprocal/divide      | 4      |
| exponential/power      | 5      |
| length/equals          | 6      |
| not/does not equal     | 7      |
| depth/matches          | 8      |
| does not match         | 9      |
| less than              | 10     |
| less than or equal     | 11     |
| ceiling/max            | 12     |
| floor/min              | 13     |
| deduplicate/replicate  | 14     |
| transpose/chunks       | 15     |
| classify/select        | 16     |
| <kaunan>               | 17     |
| drop                   | 18     |
| take                   | 19     |
| absolute value/modulus | 20     |
| reverse                | 21     |
| join                   | 22     |
| first/index            | 23     |
| range/windows          | 24     |
| <sowilo>               | 25     |
| grade                  | 26     |

# Unary Modifier

| type     | tag | data   |
| -------- | --- | ------ |
| operator | 32  | opcode |

## Opcodes

| name      | opcode |
| --------- | ------ |
| scan      | 32     |
| fold      | 33     |
| table     | 34     |
| each      | 35     |
| constant  | 36     |
| both/flip | 37     |

# Binary Modifier

| type     | tag | data   |
| -------- | --- | ------ |
| operator | 40  | opcode |

## Opcodes

| name     | opcode |
| -------- | ------ |
| over     | 48     |
| beside   | 49     |
| <mannaz> | 50     |
| choose   | 51     |
| catch    | 52     |