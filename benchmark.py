#!/usr/bin/env python3

from contextlib import contextmanager
import cProfile
from util.cprofile import capture_stats_output, parse

import adaptive_profiler

from benchmark import matmul, xml

from time import perf_counter, time_ns


class Timer:
    def __init__(self, label: str) -> None:
        self.label = label

    def __enter__(self):
        self.start_time = time_ns()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.end_time = time_ns()
        self.total_time = self.end_time - self.start_time


@contextmanager
def profiler():
    adaptive_profiler.enable()
    try:
        yield
    finally:
        adaptive_profiler.disable()


print("Matrix multiplication")

A, B = matmul.random_matrices(120, 100, 50)

no_profiling_timer = Timer('No profiling')
with no_profiling_timer:
    C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

print()

cprofile_timer = Timer('cProfile')
profile = cProfile.Profile(timer=perf_counter)
with cprofile_timer:
    with profile:
        C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

print('cProfile stats')
output = capture_stats_output(profile)
stats = parse(output)
for stat in stats:
    print(stat)

cprofile_stats = [stats[0], stats[1], stats[2]]

print()

adaprof_timer = Timer('Adaptive profiler')
with adaprof_timer:
    with profiler():
        C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

print('Adaptive profiler stats')
stats = adaptive_profiler.get_statistics()
for stat in stats:
    print(stat)

adaprof_stats = [stats[1], stats[2], stats[0]]

print()

print('Time percentages')

for stat in cprofile_stats:
    print(f'{stat.name}:', stat.cumulative_time / cprofile_timer.total_time)

for stat in adaprof_stats:
    print(f'{stat.name}:', stat.cumulative_time / adaprof_timer.total_time)

# print("XML parsing")

# with adaptive_profiler():
#     tree = xml.parse_countries()

# adaptive_profiler.print_statistics()
