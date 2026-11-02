"""Derive scenarios from CPython's Lib/test/test_re.py (external oracle input).

Collects re-rooted expressions whose arguments are all constants, plus the
pattern/template error checks, and writes them as a scenario program.
usage: extract_test_re.py STDLIB OUT.py

STDLIB is the pinned CPython's standard library directory
(sysconfig.get_path('stdlib')). Nothing extracted is committed: the program
is written to OUT.py, under reports/, at gate time.
"""
import ast, os, sys
from common import PRELUDE

SRC = os.path.join(sys.argv[1], 'test', 'test_re.py')
tree = ast.parse(open(SRC).read())
FLAGS = {'A','I','L','M','S','X','U','ASCII','IGNORECASE','LOCALE','MULTILINE','DOTALL','VERBOSE','UNICODE','NOFLAG','DEBUG'}
SAFE_FUNCS = {'chr', 'str', 'bytes', 'bytearray', 'memoryview', 'len', 'list', 'tuple', 'ord', 'range', 'int', 'repr', 'type'}
EXCLUDE_ATTRS = {'Scanner', 'scanner', '_parser', '_compiler', 'template', 'TEMPLATE', 'T', 'DEBUG', '_casefix', '_constants', 'sre_compile', 'sre_parse', '__reduce__', '__reduce_ex__'}

def const_ok(node):
    if isinstance(node, ast.Constant):
        return True
    if isinstance(node, (ast.Tuple, ast.List)):
        return all(const_ok(e) for e in node.elts)
    if isinstance(node, ast.UnaryOp):
        return const_ok(node.operand)
    if isinstance(node, ast.BinOp):
        return const_ok(node.left) and const_ok(node.right)
    if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == 're' and node.attr in FLAGS:
        return node.attr != 'DEBUG'
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id in SAFE_FUNCS:
        return all(const_ok(a) for a in node.args) and not node.keywords
    if isinstance(node, ast.Lambda):
        return lambda_ok(node)
    if is_re_expr(node):
        return True
    return False

def lambda_ok(node):
    # lambda m: <expr using only m and constants>
    names = {a.arg for a in node.args.args}
    def ok(n):
        if isinstance(n, ast.Name):
            return n.id in names or n.id in SAFE_FUNCS
        if isinstance(n, (ast.Constant,)):
            return True
        return all(ok(c) for c in ast.iter_child_nodes(n) if not isinstance(c, (ast.expr_context, ast.operator, ast.unaryop, ast.cmpop, ast.boolop)))
    return ok(node.body)

def is_re_expr(node):
    """An expression chain rooted at the `re` module with constant arguments."""
    if isinstance(node, ast.Name):
        return node.id == 're'
    if isinstance(node, ast.Attribute):
        if node.attr in EXCLUDE_ATTRS:
            return False
        return is_re_expr(node.value)
    if isinstance(node, ast.Call):
        if not is_re_expr(node.func):
            return False
        return all(const_ok(a) for a in node.args) and all(const_ok(k.value) and k.arg for k in node.keywords)
    if isinstance(node, ast.Subscript):
        return is_re_expr(node.value) and const_ok(node.slice)
    return False

def has_call(node):
    return any(isinstance(n, ast.Call) for n in ast.walk(node))

exprs = []
seen = set()
parents = {}
for node in ast.walk(tree):
    for child in ast.iter_child_nodes(node):
        parents[child] = node

def outermost(node):
    while True:
        p = parents.get(node)
        if p is not None and isinstance(p, (ast.Attribute, ast.Call, ast.Subscript)) and is_re_expr(p) and (not isinstance(p, ast.Call) or p.func is node or node in p.args):
            if isinstance(p, ast.Call) and node in p.args:
                break
            node = p
            continue
        return node
    return node

for node in ast.walk(tree):
    if isinstance(node, ast.Call) and is_re_expr(node) and has_call(node):
        top = outermost(node)
        src = ast.unparse(top)
        if src not in seen and 're.' in src:
            seen.add(src)
            exprs.append(src)
    # error checks
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
        name = node.func.attr
        args = node.args
        if name == 'checkPatternError' and args and const_ok(args[0]):
            flags = ''
            for k in node.keywords:
                if k.arg == 'flags' and const_ok(k.value):
                    flags = ', ' + ast.unparse(k.value)
            src = 're.compile(%s%s)' % (ast.unparse(args[0]), flags)
            if src not in seen:
                seen.add(src); exprs.append(src)
        if name == 'checkTemplateError' and len(args) >= 3 and all(const_ok(a) for a in args[:3]):
            src = 're.sub(%s, %s, %s)' % tuple(ast.unparse(a) for a in args[:3])
            if src not in seen:
                seen.add(src); exprs.append(src)
        if name in ('assertRaises', 'assertRaisesRegex') and len(args) >= 2:
            fn_index = 1 if name == 'assertRaises' else 2
            if len(args) > fn_index and is_re_expr(args[fn_index]) and all(const_ok(a) for a in args[fn_index+1:]):
                src = '%s(%s)' % (ast.unparse(args[fn_index]), ', '.join(ast.unparse(a) for a in args[fn_index+1:]))
                if src not in seen:
                    seen.add(src); exprs.append(src)

out = sys.argv[2]
lines = [PRELUDE]
srcs = []
for i, e in enumerate(exprs):
    lines.append('run(%d, lambda: %s)' % (i, e))
    srcs.append('%d %s' % (i, e.replace('\n', '\\n')))
open(out, 'w').write('\n'.join(lines) + '\n')
open(out.replace('.py', '.src'), 'w').write('\n'.join(srcs) + '\n')
print(len(exprs), 'scenarios')
