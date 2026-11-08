# Basic Block IR
Basic Block IR should be super easy to parse, because I don't like writing parsers.

Each BB contains a label, an arbitrary amount of operations, and a finalizer.
A BB is part of a group that shares a stack frame, like a function would.

Example:
```bbir
l fib:in index
o set one 1
o le is_le index one
f br is_le fib:base fib:rec1

l fib:base
f return one

l fib:rec1
o set n1 -1
o add min1 index n1
f call fib:rec2 fib:in min1

l fib:rec2
o set res1 ret
o set n2 -2
o add min2 index n2
f call fib:rec3 fib:in min2

l fib:rec3
o set res2 ret
o add res res1 res2
f return res

l main:in
o getc input
f call main:end fib:in input

l main:end
o putc ret
f return
```
