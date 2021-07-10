# TO(...)

Generic all purpose static type converter for python! One stop shop!
The tool takes functions registered with their input and output types (plus optionally some extra context if needed). Then builds a graph for converting optimally through those types.
Providing a simple way to create basic constructors for classes, as well as a means to easily work in a more type-centric way without the hassle of leaping through nested hoops.

NOTE: Extreme experimental warning. Don't use in production. However DO feel free to mess around with the concept, and even reach out with any ideas or thoughts.


``` python

    from to import to
    assert to("123", int) == 123

```

Not impressed? Ok, how about you add your own classes!

``` python

    import to

    class MyClass:
        def __init__(self, value: str): ...

    to.add_conversion(1, str, (), MyClass, ())

    assert to("123", MyClass) == MyClass("123")
```

Or maybe there are some dependencies with wrapped classes?

``` python

    class MyWrappedClass:
        def __init__(self, other: MyClass): ...

    to.add_conversion(1, MyClass, (), MyWrappedClass, ())

    assert to("123", MyWrappedClass) == MyWrappedClass(MyClass("123"))
```

Oh damn, MyClass takes a number as a string, but all I have is an int

``` python

    assert to(123, MyWrappedClass) == MyWrappedClass(MyClass("123"))
```

