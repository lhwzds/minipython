"""A finite CPU workload that a low MiniPython wall-clock limit stops."""

total = 0
for value in range(1_000_000):
    total += value
print(total)
