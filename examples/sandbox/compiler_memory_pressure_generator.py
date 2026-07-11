"""Generate a finite source file that puts memory pressure on the compiler."""

print("values = [" + "0," * 120_000 + "]\nprint(len(values))")
