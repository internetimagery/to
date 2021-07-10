import unittest

from to import to


class TestBasics(unittest.TestCase):

    def test_int(self):
        self.assertEqual(to("123", int), 123)

    def test_str(self):
        self.assertEqual(to(123, str), "123")
        self.assertEqual(to(123.0, str), "123.0")

    def test_bool(self):
        self.assertEqual(to(123, bool), True)
        self.assertEqual(to("", bool), False)
