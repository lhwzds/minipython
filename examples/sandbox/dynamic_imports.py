"""Show that dynamic execution cannot bypass the import allowlist."""

probes = [
    ("eval-import", lambda: eval("__import__('socket')")),
    ("exec-import", lambda: exec("import subprocess")),
    ("compiled-import", lambda: exec(compile("import os", "<payload>", "exec"))),
]

for label, probe in probes:
    try:
        probe()
    except ModuleNotFoundError:
        print(label, "blocked")
    else:
        print(label, "available")
