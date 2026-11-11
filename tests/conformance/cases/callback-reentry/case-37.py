state = {}


async def source():
    await state['coroutine'].send(None)


state['coroutine'] = source()
try:
    state['coroutine'].send(None)
except ValueError as error:
    print('caught', type(error).__name__, error)
