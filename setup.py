from setuptools import setup

with open("README.md") as h:
    desc = h.read()

setup(
    setup_requires=['setuptools_scm'],
    use_scm_version=True,
    name = "to",
    description="Automatic type conversion framework",
    long_description=desc,
    author="Jason Dixon",
    author_email="jason.dixon.email@gmail.com",
    url="https://github.com/internetimagery/to",
    packages=["to"],
    package_data={"to": ["py.typed"]},
)
