from to._internal import Conversions, ConversionError

__all__ = ("to", "add_conversion", "add_revealer", "ConversionError", "Conversions")


_GLOBAL_REGISTRY = Conversions()

to = _GLOBAL_REGISTRY.convert
add_revealer = _GLOBAL_REGISTRY.add_revealer
add_conversion = _GLOBAL_REGISTRY.add_conversion


def shield(*types):
    def decorator(func):
        from functools import wraps

        @wraps(func)
        def wrapper(*args, **kwargs):
            return func(*map(to, args, types), **kwargs)
        return wrapper
    return decorator


def _initialize_builtins():
    """
    Initialize some basic conversions between built in types
    """
    from itertools import permutations

    for source, target in permutations((str, int, float, bool), 2):
        add_conversion(1, source, (), target, (), target)

    # TODO: Consider support for more generic types. So conversions can happen
    # within container types. eg convert List[str] to List[int]

_initialize_builtins()
del _initialize_builtins



