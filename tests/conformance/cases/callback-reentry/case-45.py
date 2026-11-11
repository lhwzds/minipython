def run(target):
    state = {}

    async def source():
        if target == 'send':
            state['coroutine'].send(5)
        elif target == 'throw':
            state['coroutine'].throw(ValueError('x'))
        else:
            state['coroutine'].close()

    coroutine = source()
    state['coroutine'] = coroutine
    try:
        coroutine.send(None)
    except ValueError as error:
        print(target, type(error).__name__, error)


run('send')
run('throw')
run('close')
