import sys
def show(label, e):
    print(label, repr(e))
def run(exc, T, named=True):
    match = rest = info = None
    try:
        try:
            raise exc
        except* T as e:
            info = sys.exc_info()[1]
            match = e
    except BaseException as e:
        rest = e
    print(repr(match), repr(rest), info is match)
run(ExceptionGroup("test1", [ValueError("V"), TypeError("T")]), SyntaxError)
run(ExceptionGroup("test2", [ValueError("V"), TypeError("T")]), (ValueError, TypeError))
run(ExceptionGroup("test3", [ValueError("V"), TypeError("T")]), ValueError)
nested = ExceptionGroup("t", [ValueError(1), ExceptionGroup("in", [TypeError(2), ValueError(3)]), ExceptionGroup("in2", [TypeError(4)])])
run(nested, ValueError)
run(nested, TypeError)
run(nested, (TypeError, ValueError))
run(nested, Exception)
run(ValueError("naked"), ValueError)
run(ValueError("naked"), TypeError)
run(KeyboardInterrupt("ki"), KeyboardInterrupt)
run(ExceptionGroup("eg", [ValueError(1)]), Exception)
run(BaseExceptionGroup("beg", [KeyboardInterrupt(1), ValueError(2)]), ValueError)
run(BaseExceptionGroup("beg", [KeyboardInterrupt(1), ValueError(2)]), KeyboardInterrupt)
run(BaseExceptionGroup("beg", [KeyboardInterrupt(1), ValueError(2)]), BaseException)
print("--- multiple")
try:
    raise ExceptionGroup("mmu", [OSError("os"), BlockingIOError("io")])
except* BlockingIOError as e:
    print("io", repr(e))
except* OSError as e:
    print("os", repr(e))
try:
    raise ExceptionGroup("fst", [BlockingIOError("io")])
except* OSError as e:
    print("first", repr(e))
except* BlockingIOError:
    print("never")
print("--- nested")
def inner_star_clears():
    try:
        raise ExceptionGroup("outer", [LookupError("a")])
    except* LookupError:
        try:
            raise ExceptionGroup("inner", [ArithmeticError("b")])
        except* ArithmeticError:
            print("inner handled", repr(sys.exc_info()[1]))
        print("back in outer", repr(sys.exc_info()[1]))
inner_star_clears()
seen = []
for attempt in range(3):
    try:
        raise ExceptionGroup("round %d" % attempt, [IndexError(attempt)])
    except* IndexError as grp:
        seen.append(grp.exceptions[0].args[0])
print("rounds", seen, sys.exc_info())
print("--- reraise")
def outcome(label, func):
    try:
        func()
    except BaseException as caught:
        leaves = [repr(x) for x in caught.exceptions] if isinstance(caught, BaseExceptionGroup) else None
        print(label, type(caught).__name__, repr(caught), leaves)
    else:
        print(label, "no exception")
batch = lambda: ExceptionGroup("batch", [KeyError("k"), IndexError("i"), ZeroDivisionError("z")])
def keep_both():
    try:
        raise batch()
    except* KeyError:
        raise
    except* IndexError:
        raise
def keep_one_drop_one():
    try:
        raise batch()
    except* IndexError:
        pass
    except* KeyError:
        raise
def keep_one_of_two():
    try:
        raise ExceptionGroup("pair", [KeyError("k"), IndexError("i")])
    except* IndexError:
        pass
    except* KeyError:
        raise
def drop_all():
    try:
        raise ExceptionGroup("pair", [KeyError("k"), IndexError("i")])
    except* (KeyError, IndexError):
        pass
def naked_bare_raise():
    try:
        raise IndexError("alone")
    except* IndexError:
        raise
for label, func in [("keep-both", keep_both), ("keep-one-drop-one", keep_one_drop_one), ("keep-one-of-two", keep_one_of_two), ("drop-all", drop_all), ("naked", naked_bare_raise)]:
    outcome(label, func)
print("--- raise")
def new_in_one_clause():
    source = ExceptionGroup("src", [KeyError("k"), ZeroDivisionError("z")])
    try:
        raise source
    except* ZeroDivisionError:
        raise RuntimeError("replacement")
def new_in_tuple_clause():
    try:
        raise ExceptionGroup("src", [KeyError("k"), IndexError("i")])
    except* (KeyError, IndexError):
        raise NotImplementedError("tuple")
def new_in_two_clauses():
    try:
        raise ExceptionGroup("src", [KeyError("k"), IndexError("i")])
    except* KeyError:
        raise RuntimeError("from-key")
    except* IndexError:
        raise RuntimeError("from-index")
def chained(explicit):
    try:
        raise ExceptionGroup("src", [KeyError("k"), IndexError("i")])
    except* KeyError as part:
        if explicit:
            raise RuntimeError("caused") from part
        raise RuntimeError("implicit")
def show_links(label, func):
    try:
        func()
    except BaseException as caught:
        if isinstance(caught, BaseExceptionGroup):
            for leaf in caught.exceptions:
                suppressed = None if isinstance(leaf, BaseExceptionGroup) else leaf.__suppress_context__
                print(label, repr(caught.message), repr(leaf), repr(leaf.__cause__), repr(leaf.__context__), suppressed)
        else:
            print(label, repr(caught), repr(caught.__cause__), repr(caught.__context__))
show_links("one", new_in_one_clause)
show_links("tuple", new_in_tuple_clause)
show_links("two", new_in_two_clauses)
show_links("explicit", lambda: chained(True))
show_links("implicit", lambda: chained(False))
def traceback_kept():
    source = ExceptionGroup("tb", [KeyError("k"), ZeroDivisionError("z")])
    try:
        try:
            raise source
        except* ZeroDivisionError:
            raise RuntimeError("r")
    except ExceptionGroup as caught:
        context = caught.exceptions[0].__context__
        print("tb", context.__traceback__ is source.__traceback__, caught.__traceback__ is source.__traceback__)
traceback_kept()
print("--- finally/else")
def fe(exc):
    try:
        try:
            if exc: raise exc
        except* ValueError as e:
            print("handled", repr(e))
        else:
            print("else")
        finally:
            print("finally")
    except BaseException as e:
        print("out", repr(e))
fe(None)
fe(ExceptionGroup("g", [ValueError(1), KeyError(2)]))
fe(ValueError(3))
fe(KeyError(4))
def ret():
    try:
        return "body-return"
    except* ValueError:
        pass
    finally:
        print("ret finally")
print(ret())
print("--- errors in matcher")
def bad(m):
    try:
        try:
            raise ExceptionGroup("eg", [ValueError(1)])
        except* ValueError:
            print("first")
        except* m:
            print("no")
    except Exception as e:
        print(type(e).__name__, e, repr(e.__context__))
bad(42)
bad(ExceptionGroup)
bad((ValueError, (TypeError,)))
def undefined_matcher():
    try:
        try:
            raise ExceptionGroup("eg", [ValueError(1)])
        except* undefined_name:
            pass
    except NameError as e:
        print("NameError", e, repr(e.__context__))
undefined_matcher()
print(sys.exc_info())
