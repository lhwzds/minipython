import json

def emit(kind, **fields):
    fields['kind'] = kind
    print(json.dumps(fields, sort_keys=True, ensure_ascii=True))

def emit_calls(logger):
    emit('calls', calls=logger.calls)

def emit_exception(exc):
    emit('exception', type=type(exc).__name__, message=str(exc))
