from to._internal import Conversions, ConversionError

__all__ = ("to", "add_conversion", "ConversionError", "Conversions")


_CONVERSIONS = Conversions()

def to(value, type_want, variations_want=None, type_have=None, variations_have=None, explicit=False):
    """
    Convert from one type into another.

    >>> assert to("123", int) == 123
    """
    return _CONVERSIONS.convert(
        value,
        type_want,
        () if variations_want is None else variations_want,
        type(value) if type_have is None else type_have,
        () if variations_have is None else variations_have,
        explicit,
    )


def add_conversion(
    function,
    cost,
    type_in,
    type_out,
    variations_in=None,
    variations_out=None,
):
    """
    Add a converter that can later be used to convert between defined types.

    >>> add_conversion(int, 0, str, int)
    >>> assert to("123", int) == 123
    """
    _CONVERSIONS.add_conversion(
        cost,
        type_in,
        () if variations_in is None else variations_in,
        type_out,
        () if variations_out is None else variations_out,
        function,
    )


def _initialize_builtins():
    """
    Initialize some basic conversions between built in types
    """
    cast_map = [
        (a, b)
        for a in (str, int, float, bool)
        for b in (str, int, float, bool)
    ]
    for source, target in cast_map:
        add_conversion(target, 1, source, target)



_initialize_builtins()



