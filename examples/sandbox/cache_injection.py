"""Show that sys.modules injection cannot bypass the import allowlist."""

import sys

sys.modules["socket"] = "injected"
try:
    __import__("socket")
except ModuleNotFoundError:
    print("socket blocked")
else:
    print("socket available")
