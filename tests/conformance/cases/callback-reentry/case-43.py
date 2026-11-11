def run(target):
    state = {}

    async def nested():
        if target == 'anext':
            await state['generator'].__anext__()
        elif target == 'athrow':
            await state['generator'].athrow(ValueError('x'))
        else:
            await state['generator'].aclose()

    async def source():
        await nested()
        yield 1

    generator = source()
    state['generator'] = generator

    async def main():
        try:
            await generator.__anext__()
        except RuntimeError as error:
            print(target, type(error).__name__, error)
        except BaseException as error:
            print(target, 'other', type(error).__name__, error)

    step = main()
    try:
        step.send(None)
    except StopIteration:
        pass
    try:
        step.send(None)
    except StopIteration:
        pass
    except BaseException as error:
        print(target, 'main', type(error).__name__, error)


run('anext')
run('athrow')
run('aclose')
