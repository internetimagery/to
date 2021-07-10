from to._internal import Conversions, ConversionError

__all__ = ("to", "add_conversion", "add_revealer", "ConversionError", "Conversions")


_CONVERSIONS = Conversions()

to = _CONVERSIONS.convert

def add_revealer(function, type_in):
    """
    Add a function that can interpret more context from an input.
    """
    _CONVERSIONS.add_revealer(type_in, function)


def add_conversion(
    cost,
    type_in,
    type_out,
    function,
    variations_in=None,
    variations_out=None,
):
    """
    Add a converter that can later be used to convert between defined types.

    >>> add_conversion(0, str, int, int)
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
        add_conversion(1, source, target, target)

    # TODO: Consider support for more generic types. So conversions can happen
    # within container types. eg convert List[str] to List[int]



_initialize_builtins()



