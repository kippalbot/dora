"""
Setup script for dora-common package.
"""

from setuptools import setup, find_packages

setup(
    name="dora-common",
    version="0.1.0",
    description="Common utilities for Dora nodes",
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        "pyarrow>=10.0.0",
    ],
    author="Dora Team",
    author_email="team@dora.ai",
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
    ],
)