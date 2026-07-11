"""A finite workload that a low MiniPython instruction budget stops."""

total = 0
for value in range(1000):
    total += value
print(total)
