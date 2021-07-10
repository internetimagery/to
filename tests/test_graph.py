from __future__ import print_function

import unittest
import os, sys, itertools

sys.path.append(os.path.join(os.path.dirname(os.path.dirname(__file__)), "target", "release"))
from to import Conversions, ConversionError

module = sys.modules[__name__]


class Executor(object):
    def __init__(self, variation=""):
        self.variation = variation and ":" + variation

    def __call__(self, value):
        return value + " -> " + self.__class__.__name__ + self.variation


letters = "ABCDEFG"

for letter in letters:
    setattr(module, "TYPE_" + letter, type("TYPE_" + letter, (Executor,), {}))

for a, b in itertools.permutations(letters, 2):
    setattr(module, a + "to" + b, type(a + "to" + b, (Executor,), {}))


def bad_transmuter(_):
    raise RuntimeError("BAD STUFF")


class TestGraph(unittest.TestCase):
    def setUp(self):
        self.conv = Conversions()

    def test_basic_graph(self):
        # A - B - C - D
        # |         /
        # E - F - G

        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_A, [], TYPE_E, [], AtoE())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(1, TYPE_C, [], TYPE_D, [], CtoD())
        self.conv.add_conversion(1, TYPE_E, [], TYPE_F, [], EtoF())
        self.conv.add_conversion(1, TYPE_F, [], TYPE_G, [], FtoG())
        self.conv.add_conversion(1, TYPE_G, [], TYPE_D, [], GtoD())

        self.assertEqual(
            self.conv.convert("start", TYPE_D, [], TYPE_A),
            "start -> AtoB -> BtoC -> CtoD",
        )

    def test_revealer(self):
        # A - B - C
        #  \     /
        #   - D'-

        def activator(_):
            yield "var"
        self.conv.add_revealer(TYPE_A, activator)
        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_A, ["var"], TYPE_D, [], AtoD("var"))
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(1, TYPE_D, [], TYPE_C, [], DtoC())

        self.assertEqual(
            self.conv.convert("start", TYPE_C, [], TYPE_A),
            "start -> AtoD:var -> DtoC",
        )


    def test_join_graph(self):
        # A           E
        #  \         /
        #   - C - D -
        #  /         \
        # B           F

        self.conv.add_conversion(1, TYPE_A, [], TYPE_C, [], AtoC())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(1, TYPE_C, [], TYPE_D, [], CtoD())
        self.conv.add_conversion(1, TYPE_D, [], TYPE_E, [], DtoE())
        self.conv.add_conversion(1, TYPE_D, [], TYPE_F, [], DtoF())

        self.assertEqual(
            self.conv.convert("start", TYPE_F, [], TYPE_A),
            "start -> AtoC -> CtoD -> DtoF",
        )

    def test_basic_variation(self):

        # A = B = C'

        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_A, [], BtoA())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(
            1, TYPE_C, [], TYPE_B, ["var"], CtoB("var")
        )

        self.assertEqual(
            self.conv.convert("start", TYPE_A, ["var"], TYPE_A),
            "start -> AtoB -> BtoC -> CtoB:var -> BtoA",
        )

    def test_variation_preference(self):
        #     B       D'
        #    / \     / \
        # A -   - C -   - E
        #    \ /     \ /
        #     F'      G

        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_A, [], TYPE_F, [], AtoF())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(
            2, TYPE_C, [], TYPE_D, ["var2"], CtoD("var2")
        )
        self.conv.add_conversion(1, TYPE_C, [], TYPE_G, [], CtoG())
        self.conv.add_conversion(1, TYPE_D, [], TYPE_E, [], DtoE())
        self.conv.add_conversion(
            1, TYPE_F, [], TYPE_C, ["var1"], FtoC("var1")
        )
        self.conv.add_conversion(1, TYPE_G, [], TYPE_E, [], GtoE())

        self.assertEqual(
            self.conv.convert("start", TYPE_E, ["var1", "var2"], TYPE_A),
            "start -> AtoF -> FtoC:var1 -> CtoD:var2 -> DtoE",
        )

    def test_revisit(self):

        # A - B - C - D'
        #  \  |   |   |
        #   - E - F - G

        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(1, TYPE_B, [], TYPE_E, [], BtoE())
        self.conv.add_conversion(
            3, TYPE_C, [], TYPE_D, ["var"], CtoD("var")
        )
        self.conv.add_conversion(1, TYPE_C, [], TYPE_F, [], CtoF())
        self.conv.add_conversion(1, TYPE_D, [], TYPE_G, [], DtoG())
        self.conv.add_conversion(1, TYPE_E, [], TYPE_A, [], EtoA())
        self.conv.add_conversion(1, TYPE_F, [], TYPE_E, [], FtoE())
        self.conv.add_conversion(1, TYPE_G, [], TYPE_F, [], GtoF())

        self.assertEqual(
            self.conv.convert("start", TYPE_A, ["var"], TYPE_A),
            "start -> AtoB -> BtoC -> CtoD:var -> DtoG -> GtoF -> FtoE -> EtoA",
        )

    def test_failures(self):

        # A - B
        # C - D
        # E'- F - G!

        self.conv.add_conversion(1, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(1, TYPE_C, [], TYPE_D, [], CtoD())
        self.conv.add_conversion(
            1, TYPE_E, ["var"], TYPE_F, [], EtoF("var")
        )
        self.conv.add_conversion(1, TYPE_F, [], TYPE_G, [], bad_transmuter)

        self.assertEqual(
            self.conv.convert("start", TYPE_F, [], TYPE_E, ["var"]),
            "start -> EtoF:var",
        )
        with self.assertRaises(TypeError):
            self.conv.convert("start", TYPE_D, [], TYPE_A)

        with self.assertRaises(TypeError):
            self.conv.convert("start", TYPE_F, [], TYPE_E)

        with self.assertRaises(ConversionError):
            self.conv.convert("start", TYPE_G, [], TYPE_F)

    def test_redirect(self):

        # A - B - C
        #  \     /
        #   - D!-

        self.conv.add_conversion(3, TYPE_A, [], TYPE_B, [], AtoB())
        self.conv.add_conversion(3, TYPE_B, [], TYPE_C, [], BtoC())
        self.conv.add_conversion(1, TYPE_A, [], TYPE_D, [], bad_transmuter)
        self.conv.add_conversion(1, TYPE_D, [], TYPE_C, [], DtoC())

        self.assertEqual(
            self.conv.convert("start", TYPE_C, [], TYPE_A, []),
            "start -> AtoB -> BtoC",
        )


if __name__ == "__main__":
    unittest.main()
