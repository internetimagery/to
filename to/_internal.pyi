from typing import (
    Any,
    Callable,
    Hashable,
    Iterator,
    Sequence,
    Type,
    TypeVar,
)

A = TypeVar("A")
B = TypeVar("B")

class ConversionError: ...

class Conversions:

    def add_conversion(
        self,
        cost: int,
        type_in: Type[A],
        variations_in: Sequence[Hashable],
        type_out: Type[B],
        variations_out: Sequence[Hashable],
        function: Callable[[A], B],
    ) -> None:
        ...

    def add_revealer(
        self,
        type_in: Type[A],
        function: Callable[[A], Iterator[Hashable]],
    ) -> None:
        ...

    def convert(
        self,
        value: A,
        type_want: Type[B],
        variations_want: Sequence[Hashable],
        type_have: Type[A],
        variations_have: Sequence[Hashable],
        explicit: bool,
    ) -> B:
        ...
