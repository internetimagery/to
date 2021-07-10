import unittest

from to import to, shield


class TestBasics(unittest.TestCase):

    def test_int(self):
        self.assertEqual(to("123", int), 123)

    def test_str(self):
        self.assertEqual(to(123, str), "123")
        self.assertEqual(to(123.0, str), "123.0")

    def test_bool(self):
        self.assertEqual(to(123, bool), True)
        self.assertEqual(to("", bool), False)

class TestShield(unittest.TestCase):

    def test_int(self):

        @shield(int)
        def test(num):
            return num

        self.assertEqual(test(123), 123)
        self.assertEqual(test("123"), 123)
        self.assertEqual(test("abc"), 1)

    def test_bool(self):

        @shield(bool, bool)
        def test(cond1, cond2):
            return cond1, cond2

        self.assertEqual(test(True, False), (True, False))
        self.assertEqual(test("", "yep"), (False, True))
