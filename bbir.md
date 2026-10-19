# Basic Block IR
Basic Block IR should be super easy to parse, because I don't like writing parsers.

Each BB contains a label, an arbitrary amount of operations, and a finalizer.
A BB is part of a group that shares a stack frame, like a function would.

Example:
```bbir
l fib:in 1 index
o le is_le index 1
f br is_le fib:base fib:rec1

l fib:base
f return 1

l fib:rec1
o add min1 index -1
f call fib:rec2 fib:in 1 min1

l fib:rec2
o set res1 ret
o add min2 index -2
f call fib:rec3 fib:in 1 min2

l fib:rec3
o set res2 ret
o add res res1 res2
f return res

l main:in
o getc input
f call main:out fib:in 1 input

l main:end
o putc ret
f return
```
