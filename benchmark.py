#!/usr/bin/env python3

from contextlib import contextmanager
import adaptive_profiler
import cProfile

from benchmark import matmul, xml

import sys
from time import time_ns


@contextmanager
def timer(label):
    start_time = time_ns()
    try:
        yield
    finally:
        end_time = time_ns()
        total_time = end_time - start_time
        total_time_ms = total_time / 1_000_000
        print(f"{label}: {total_time_ms}ms")


@contextmanager
def profiler():
    adaptive_profiler.enable()
    try:
        yield
    finally:
        adaptive_profiler.disable()


print("Matrix multiplication")

A, B = matmul.random_matrices(120, 50, 30)

with timer('No profiling'):
    C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

cprofiler = cProfile.Profile()
with timer('cProfile'):
    with cprofiler:
        C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

with timer('Adaptive profiler'):
    with profiler():
        C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

# adaptive_profiler.print_statistics()

# print()

# print("XML parsing")

# with adaptive_profiler():
#     tree = xml.parse_countries()

# adaptive_profiler.print_statistics()
