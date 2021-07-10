# TO(...) [![Build Status](https://travis-ci.com/internetimagery/to.svg?branch=develop)](https://travis-ci.com/internetimagery/to)

Generic all purpose semi-static type converter for python! For automatic, but meaningful conversions.

``` python

    from to import shield, add_conversion, to

    @shield(str, str)
    def concat(prefix, suffix):
    	return prefix + suffix

    assert concat("one ", 23) == "one 23"

    concat([1,2,3], 456)
    # TypeError: Could not convert [] to <class 'str'>. Perhaps some conversion steps are missing.

    add_conversion(1, list, (), str, (), lambda x: "".join(map(lambda y: to(y, str), x)))

    assert concat([1,2,3], 456) == "123456"
```

The tool takes functions registered with their input and output types, and cost of computation (plus optionally some extra context). Then builds a graph for converting optimally and meaninfully through those types.

This provides an alternate way to create basic constructors for classes, as well as a means to easily work in a more type-centric way without the hassle of leaping through nested hoops.


NOTE: Extreme experimental warning. Do not use timidly in production. However DO feel free to mess around with the tool, the concept, and even reach out with any ideas or thoughts.

## How To...


``` python

    from to import *
    assert to(123, str) == "123" # Drumroll!

```

Not impressed? Ok, how about you add your own classes!

``` python

    @dataclass
    class Age:
        value: int

    # Cost, In_Type, In_Context, Out_Type, Out_Context, Callable
    add_conversion(1, int, (), Age, (), Age)

    assert to(18) == Age(18)
```

Or maybe you've wrapped a class with a dependency?

``` python

    @dataclass
    class Person:
        age: Age

    add_conversion(1, Age, (), Person, (), Person)

    assert to(18, Person) == Person(Age(18))
```

And back again!

``` python

    from operator import attrgetter

    add_conversion(1, Age, (), int, (), attrgetter("value"))
    add_conversion(1, Person, (), Age, (), attrgetter("age"))

    assert to(Person(Age(18)), str) == "18"
```

Units anyone?

``` python

    @dataclass
    class Centemeter:
        value: float
    
    @dataclass
    class Meter:
        value: float

    add_conversion(2, Meter, (), Centemeter, (), lambda x: Centemeter(x.value * 100))

    assert to(Meter(2), Centemeter) == Centemeter(200)
```

Oh damn, Age takes a number as an int, but I have a str...

``` python

    assert to("123", Person) == Person(Age(123))
```

Also... I have a bool... not sure this is what I want... but it works!

``` python

    assert to(True, Person) = Person(Age(1))
```

Now we're getting into the weeds... can't convert the string directly to a number, but can if we make it a bool first!

Life finds a way!

``` python

    val = to("not a number", Person)
    # Warning: ValueError: invalid literal for int() with base 10: 'not a number'
    assert val = Person(Age(1))
```

Yeah... be careful with that!
