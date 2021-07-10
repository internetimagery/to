from setuptools import setup

with open("README.md") as h:
    desc = h.read()

setup(
    name = "to",
    version="0.0.1",
    description="Automatic type conversion framework",
    long_description=desc,
    author="Jason Dixon",
    author_email="jason.dixon.email@gmail.com",
    url="https://github.com/internetimagery/to",
    packages=["to"],
)
