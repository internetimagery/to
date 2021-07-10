# TO(...)

Generic all purpose static type converter for python! One stop shop!

The tool takes functions registered with their input and output types (plus optionally some extra context). Then builds a graph for converting optimally through those types.

This provides an alternate way to create basic constructors for classes, as well as a means to easily work in a more type-centric way without the hassle of leaping through nested hoops.

NOTE: Extreme experimental warning. Don't use in production. However DO feel free to mess around with the concept, and even reach out with any ideas or thoughts.


``` python

    from to import to
    assert to(123, str) == "123" # Drumroll!

```

Not impressed? Ok, how about you add your own classes!

``` python

    import to

    class MyClass:
        def __init__(self, value: int): ...

    to.add_conversion(1, int, (), MyClass, (), MyClass)

    assert to(123, MyClass) == MyClass(123)
```

Or maybe you've wrapped a class with a dependency?

``` python

    class MyWrapperClass:
        def __init__(self, other: MyClass): ...

    to.add_conversion(1, MyClass, (), MyWrapperClass, (), MyWrapperClass)

    assert to(123, MyWrapperClass) == MyWrapperClass(MyClass(123))
```

Oh damn, MyClass takes a number as an int, but I have a str...

``` python

    assert to("123", MyWrapperClass) == MyWrapperClass(MyClass(123))
```

Also... I have a bool... not sure this is what I want... but it works!

``` python

    assert to(True, MyWrapperClass) = MyWrapperClass(MyClass(1))
```

And back again!

``` python

    from operator import attrgetter

    to.add_conversion(1, MyClass, (), int, (), attrgetter("value"))
    to.add_conversion(1, MyWrapperClass, (), MyClass, (), attrgetter("value"))

    assert to(MyWrapperClass(MyClass(123)), str) == "123"
```
