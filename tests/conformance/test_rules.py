"""The rules of tree/layer0.toml are reviewed topic labels that state no behaviour.

The recorded output of the cases is the observation, and run.py
--check-oracle compares it with CPython. A rule only names its leaf's topic.
The whole file is pinned: it must equal the header followed by the leaves of
APPROVED, in order, so a new, changed, duplicated or commented rule fails
until it is reviewed and added here. The shape and word checks are a first
filter on APPROVED itself.
"""

import re
import tomllib
import unittest
from pathlib import Path

TREE = Path(__file__).resolve().parent / 'tree' / 'layer0.toml'
HEADER = (
    '# Each leaf records behaviour of CPython 3.14.6, observed as a black box by its cases.\n'
    "# A rule names the leaf's topic; the recorded output of the cases is the observation.\n"
)
CLAIM_WORDS = {
    'is', 'are', 'was', 'were', 'be', 'returns', 'raises', 'gives', 'reads',
    'calls', 'compares', 'keeps', 'retains', 'preserves', 'propagates',
    'recognizes', 'distinct', 'equal', 'unequal', 'same', 'identical',
    'never', 'always', 'must', 'only', 'not', 'cannot', 'does', 'do',
    'keep', 'compare', 'raise', 'give', 'retain', 'preserve', 'propagate',
    'yield', 'yields', 'create', 'creates', 'fail', 'fails', 'win', 'wins',
    'happen', 'happens', 'stay', 'stays', 'match', 'matches', 'differ',
    'differs', 'copied', 'reused', 'once', 'twice', 'first',
}
APPROVED = (
    ("identity-alias", "identity", "Identity of an object with itself"),
    ("identity-containers", "identity", "Identity of container aliases and freshly constructed containers"),
    ("identity-immutable", "identity", "Identity of singletons and separately constructed immutable values"),
    ("identity-functions-classes", "identity", "Identity of functions, classes and instances"),
    ("identity-bound-method", "identity", "Identity of bound methods across accesses"),
    ("identity-descriptors", "identity", "Identity of descriptors read from a class dictionary"),
    ("identity-modules", "identity", "Identity of imported modules"),
    ("identity-generic-alias", "identity", "Identity and equality of builtin generic aliases"),
    ("identity-namedtuple-methods", "identity", "Identity of named tuple bound methods across accesses"),
    ("identity-computed-scalars", "identity", "Identity of integers and strings computed separately from one value"),
    ("identity-iterator", "identity", "Identity of an iterator under iter()"),
    ("identity-descriptor-containers", "identity", "Descriptors in containers, and containers that share one item object"),
    ("identity-alias-deepcopy", "identity", "Deep copies of generic aliases"),
    ("equality-self-calls", "rich-comparison", "Equality of an object with itself"),
    ("equality-reflected", "rich-comparison", "Reflected equality after NotImplemented"),
    ("equality-subclass-first", "rich-comparison", "Subclass priority in reflected equality"),
    ("equality-notimplemented", "rich-comparison", "Equality when equality methods return NotImplemented"),
    ("ordering-notimplemented", "rich-comparison", "Ordering when both operands decline"),
    ("inequality-default", "rich-comparison", "The default inequality method"),
    ("membership-identity", "rich-comparison", "Container membership of the stored element object itself"),
    ("sequence-identity", "rich-comparison", "List and tuple equality with shared element objects"),
    ("sequence-search-identity", "rich-comparison", "List count, index and remove with shared element objects"),
    ("dict-identity", "rich-comparison", "Dictionary lookup with the stored key object itself"),
    ("nan-comparison", "rich-comparison", "NaN in comparisons and in sequences"),
    ("equality-exception", "rich-comparison", "Exceptions raised by equality"),
    ("dict-insertion-equality-exception", "rich-comparison", "Key equality exceptions during dictionary insertion"),
    ("dict-lookup-equality-exception", "rich-comparison", "Key equality exceptions during dictionary lookup"),
    ("sequence-comparison", "rich-comparison", "Comparison of lists and tuples"),
    ("mapping-comparison", "rich-comparison", "Dictionary equality"),
    ("set-comparison", "rich-comparison", "Comparison of sets and frozen sets"),
    ("mapping-family-comparison", "rich-comparison", "Equality across dict, Counter, OrderedDict, ChainMap and UserDict"),
    ("equality-wrong-type", "rich-comparison", "Non-boolean results of equality"),
    ("equality-mutation", "rich-comparison", "Comparison methods that change the list being searched"),
    ("equality-userstring-reflected", "rich-comparison", "UserString comparisons and reflected operands"),
    ("equality-raw-result", "rich-comparison", "Non-boolean comparison results in container operations"),
    ("hash-builtins", "hash", "Hashing of immutable builtins"),
    ("hash-unhashable-builtins", "hash", "Hashing of mutable builtin containers"),
    ("hash-none", "hash", "Instances of classes that set __hash__ to None"),
    ("hash-eq-disables", "hash", "Hashing of classes that define equality without __hash__"),
    ("hash-tuple-elements", "hash", "Hashing of tuple elements"),
    ("hash-dict-insert-lookup", "hash", "Hashing during dictionary insertion and lookup"),
    ("hash-dict-setdefault", "hash", "Hashing during dictionary setdefault"),
    ("hash-dict-copy-update", "hash", "Hashing when copying, building or updating from a dictionary"),
    ("hash-set-operations", "hash", "Hashing in set insertion, membership, copy and update"),
    ("hash-changes", "hash", "Dictionaries whose keys change their hash"),
    ("hash-equal-numbers", "hash", "Hashes of numerically matching integers, floats and booleans"),
    ("hash-invalid-return", "hash", "__hash__ methods that return a non-integer"),
    ("dict-setitem-user-eq", "rich-comparison", "Dictionary insertion with colliding keys that define equality"),
    ("userdict-popitem", "rich-comparison", "UserDict popitem"),
    ("ordereddict-move-to-end", "rich-comparison", "OrderedDict move_to_end"),
    ("userdict-subclass-getitem", "rich-comparison", "UserDict subclasses that override __getitem__"),
    ("counter-eq-operand-order", "rich-comparison", "Order of key equality checks in Counter and dictionary equality"),
    ("dict-setitem-hash-once", "hash", "Hashing during dictionary item assignment"),
    ("identity-class-attribute-sets", "identity", "Identity of a frozenset class attribute across repeated access"),
    ("identity-object-kinds", "identity", "Identity of objects of many kinds, and id() of a deque"),
    ("container-mutation-during-compare", "rich-comparison", "Containers changed during comparison or search"),
    ("deque-mutation-state", "rich-comparison", "Deques changed during searches, comparisons and iteration, and deque arguments, rotation, repetition, copies, iterator reprs and the __init__ descriptor"),
    ("callback-reentry", "callback", "User callbacks that re-enter the interpreter through builtins, generators and itertools, exception context, causes, notes and tracebacks, attribute lookup through raising properties, and identity of iterators and templates, and identity and types of type aliases"),
    ("iterator-reentry", "callback", "Iterator adapters advanced by their own callbacks"),
)


def render(leaves):
    return HEADER + '\n' + '\n'.join(
        f'[[leaf]]\nid = "{leaf_id}"\nprotocol = "{protocol}"\nrule = "{rule}"\n'
        for leaf_id, protocol, rule in leaves
    )


class RuleLabelTests(unittest.TestCase):
    def test_file_is_exactly_the_approved_leaves(self):
        self.assertEqual(TREE.read_text(), render(APPROVED))

    def test_approved_leaves_parse_back_unchanged(self):
        parsed = tomllib.loads(render(APPROVED))
        self.assertEqual(set(parsed), {'leaf'})
        self.assertEqual(
            [(leaf['id'], leaf['protocol'], leaf['rule']) for leaf in parsed['leaf']],
            list(APPROVED),
        )

    def test_approved_ids_are_unique(self):
        ids = [leaf_id for leaf_id, _, _ in APPROVED]
        self.assertEqual(len(ids), len(set(ids)))

    def test_every_approved_label_has_the_shape_of_a_label(self):
        self.assertTrue(APPROVED)
        for leaf_id, protocol, rule in APPROVED:
            with self.subTest(leaf=leaf_id):
                self.assertRegex(leaf_id, r'^[a-z0-9]+(-[a-z0-9]+)*$')
                self.assertRegex(protocol, r'^[a-z0-9]+(-[a-z0-9]+)*$')
                self.assertTrue(rule.isascii())
                self.assertTrue(rule.strip())
                self.assertNotRegex(rule, r'[.;:!?]\s*$')
                self.assertNotRegex(rule, r'[.;:!?]\s')
                self.assertNotRegex(rule, r"[':=<>#\"\\]")
                words = set(re.findall(r"[a-z]+", rule.lower()))
                self.assertFalse(words & CLAIM_WORDS, sorted(words & CLAIM_WORDS))


if __name__ == '__main__':
    unittest.main()
