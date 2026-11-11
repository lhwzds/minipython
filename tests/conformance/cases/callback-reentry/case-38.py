def make():
    state = {}

    async def source():
        await state['generator'].__anext__()
        yield 1

    state['generator'] = source()
    return state['generator']


async def main():
    generator = make()
    try:
        await generator.__anext__()
    except RuntimeError as error:
        print('caught', type(error).__name__, error)


step = main()
try:
    step.send(None)
except StopIteration:
    print('done')
