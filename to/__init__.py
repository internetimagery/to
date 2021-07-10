from to._internal import Conversions, ConversionError

__all__ = ("to", "add_conversion", "add_revealer", "ConversionError", "Conversions")


_GLOBAL_REGISTRY = Conversions()

to = _GLOBAL_REGISTRY.convert
add_revealer = _GLOBAL_REGISTRY.add_revealer
add_conversion = _GLOBAL_REGISTRY.add_conversion

def _initialize_builtins():
    # type: () -> None
    """
    Initialize some basic conversions between built in types
    """
    cast_map = (
        (a, b)
        for a in (str, int, float, bool)
        for b in (str, int, float, bool)
    )
    for source, target in cast_map:
        add_conversion(1, source, (), target, (), target)

    # TODO: Consider support for more generic types. So conversions can happen
    # within container types. eg convert List[str] to List[int]

_initialize_builtins()
del _initialize_builtins



