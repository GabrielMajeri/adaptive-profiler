#!/usr/bin/env python3

from contextlib import contextmanager
import adaptive_profiler

from benchmark import matmul, xml

import sys


@contextmanager
def profiler(profilefunc):
    sys.setprofile(profilefunc)
    try:
        yield
    finally:
        sys.setprofile(None)


print("Matrix multiplication")

A, B = matmul.random_matrices(12, 17, 10)

with profiler(adaptive_profiler.profiler_callback):
    C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

adaptive_profiler.print_statistics()

print()

print("XML parsing")

with profiler(adaptive_profiler.profiler_callback):
    tree = xml.parse_countries()

adaptive_profiler.print_statistics()
