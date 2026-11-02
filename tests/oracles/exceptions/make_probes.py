import os
import sys

root = sys.argv[1]
os.makedirs(os.path.join(root, "mods"), exist_ok=True)

PROBES = {
    "p_value.py": 'raise ValueError("boom")\n',
    "p_myerr.py": "class MyErr(Exception): pass\nraise MyErr()\n",
    "p_myerr_args.py": 'class MyErr(Exception): pass\nraise MyErr("bad thing", 2)\n',
    "p_myerr_one.py": 'class MyErr(Exception): pass\nraise MyErr("bad thing")\n',
    "p_myerr_sub.py": 'class MyErr(KeyError): pass\nraise MyErr("k")\n',
    "p_myerr_str.py": 'class MyErr(ValueError):\n    def __str__(self):\n        return "custom str"\nraise MyErr("x")\n',
    "mods/mymod.py": "class ModErr(Exception):\n    pass\nclass Outer:\n    class Inner(ValueError):\n        pass\n",
    "mods/p_modexc.py": 'import mymod\nraise mymod.ModErr("from module")\n',
    "mods/p_modnested.py": 'import mymod\nraise mymod.Outer.Inner("nested")\n',
    "p_json.py": 'import json\njson.loads("{")\n',
    "p_json_catch.py": (
        "import json\n"
        "try:\n"
        "    json.loads('{\"a\": 1,}')\n"
        "except json.JSONDecodeError as e:\n"
        "    print(type(e).__name__, type(e).__module__, e.msg, e.pos, e.lineno, e.colno, repr(e.doc))\n"
        "    print(isinstance(e, ValueError), e.args)\n"
        "print(json.JSONDecodeError.__module__, json.JSONDecodeError.__mro__)\n"
        "try:\n"
        "    json.loads('[1, 2')\n"
        "except ValueError as e:\n"
        "    print(type(e).__name__, e)\n"
    ),
    "p_argparse.py": 'import argparse\nraise argparse.ArgumentTypeError("bad")\n',
    "p_argparse_err.py": 'import argparse\nraise argparse.ArgumentError(None, "bad arg")\n',
    "p_keyerror.py": 'd = {}\nd["k"]\n',
    "p_empty.py": 'raise ValueError("")\n',
    "p_empty2.py": "raise ValueError\n",
    "p_notes.py": 'e = ValueError("boom")\ne.add_note("note one")\ne.add_note("note\\ntwo")\nraise e\n',
    "p_cause.py": 'try:\n    {}["a"]\nexcept KeyError as e:\n    raise ValueError("converted") from e\n',
    "p_context.py": 'try:\n    1/0\nexcept ZeroDivisionError:\n    raise ValueError("during")\n',
    "p_suppress.py": 'try:\n    1/0\nexcept ZeroDivisionError:\n    raise ValueError("suppressed") from None\n',
    "p_unraised_cause.py": 'raise ValueError("x") from KeyError("k")\n',
    "p_nested_qual.py": 'class A:\n    class E(Exception):\n        pass\nraise A.E("q")\n',
    "p_func.py": 'def f():\n    raise TypeError("in f")\nf()\n',
    "p_eg.py": 'raise ExceptionGroup("eg", [ValueError(1), TypeError("t")])\n',
    "p_str_fails.py": 'class E(Exception):\n    def __str__(self):\n        raise RuntimeError("nope")\nraise E()\n',
    "p_stdout_then.py": 'import sys\nprint("out")\nprint("err", file=sys.stderr)\nraise ValueError("after")\n',
    "p_assert.py": 'assert 1 == 2, "msg"\n',
    "p_name.py": "undefined_name\n",
    "p_sysexit.py": 'import sys\nsys.exit("bye")\n',
    "p_reraise_chain.py": (
        "try:\n"
        "    try:\n"
        '        raise KeyError("a")\n'
        "    except KeyError as e:\n"
        '        raise TypeError("b") from e\n'
        "except TypeError:\n"
        '    raise ValueError("c")\n'
    ),
    "p_list_extend.py": "l = []\nl.extend(x for x in [1, 2])\nprint(l)\n",
    "p_unicode.py": 'b"\\xff".decode("utf-8")\n',
    "p_stopiter.py": "next(iter([]))\n",
    "p_oserror.py": "raise OSError(2, 'No such file')\n",
    "p_keyerror_tuple.py": "raise KeyError('a', 'b')\n",
    "p_keyerror_empty.py": "raise KeyError()\n",
    "p_cause_notes.py": (
        "try:\n"
        "    raise KeyError('x')\n"
        "except KeyError as e:\n"
        "    e.add_note('inner note')\n"
        "    err = ValueError('outer')\n"
        "    err.add_note('outer note')\n"
        "    raise err from e\n"
    ),
    "p_syntaxerror_raise.py": "raise SyntaxError('custom')\n",
    "p_syntaxerror_exec.py": 'exec("x y")\n',
    "p_syntaxerror_eval.py": 'eval("(1 2")\n',
    "p_syntaxerror_indent.py": 'exec("if x:\\n")\n',
    "p_syntaxerror_compile.py": 'compile("yield", "dir/name.py", "exec")\n',
    "p_syntaxerror_located.py": "raise SyntaxError('m', ('f.py', 2, 3, 'a\\tbc d', 2, 6))\n",
    "p_syntaxerror_empty.py": "raise SyntaxError()\n",
    "p_exit_class.py": "raise SystemExit(3)\n",
    "p_baseexc.py": "class B(BaseException): pass\nraise B('b')\n",
    "p_keyboard.py": "raise KeyboardInterrupt\n",
    "p_recursion_ok.py": "raise RecursionError('manual')\n",
    "p_import_missing.py": "import notamodule\n",
    "p_attr.py": "None.foo\n",
    "p_notes_nonstr.py": "e = ValueError('v')\ne.__notes__ = ['a', 3]\nraise e\n",
    "p_notes_bad.py": "e = ValueError('v')\ne.__notes__ = 42\nraise e\n",
    "p_context_cycle.py": (
        "try:\n"
        "    raise ValueError('a')\n"
        "except ValueError as a:\n"
        "    b = TypeError('b')\n"
        "    a.__context__ = b\n"
        "    raise b\n"
    ),
}

for name, source in PROBES.items():
    with open(os.path.join(root, name), "w") as handle:
        handle.write(source)
print(len(PROBES))
