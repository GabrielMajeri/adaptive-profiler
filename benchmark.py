#!/usr/bin/env python3

from contextlib import contextmanager
import cProfile
import sys
from time import perf_counter, time_ns

import adaptive_profiler
from benchmark import matmul
from util.cprofile import capture_stats_output, parse


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
        # TODO: determine why disable doesn't work
        sys.setprofile(None)


print("Matrix multiplication")

A, B = matmul.random_matrices(50, 30, 40)
N = 64

no_profiling_timer = Timer('No profiling')
with no_profiling_timer:
    for _ in range(N):
        C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

print(f'No profiling duration: {no_profiling_timer.total_time} ns')

print()

cprofile_timer = Timer('cProfile')
profile = cProfile.Profile(timer=perf_counter)
with cprofile_timer:
    with profile:
        for _ in range(N):
            C = matmul.multiply_matrices(A, B)

matmul.verify_result(A, B, C)

print(f'cProfile duration: {cprofile_timer.total_time} ns')
print('cProfile stats')
output = capture_stats_output(profile)
stats = parse(output)
for stat in stats:
    print(stat)

cprofile_stats = stats

print()

adaprof_timer = Timer('Adaptive profiler')
with adaprof_timer:
    with profiler():
        for _ in range(N):
            C = matmul.multiply_matrices(A, B)
            adaptive_profiler.update()

matmul.verify_result(A, B, C)

print(f'Adaptive profiler duration: {adaprof_timer.total_time} ns')
print('Adaptive profiler stats')
stats = adaptive_profiler.get_statistics()
for stat in stats:
    print(stat)

adaprof_stats = stats

print()

print('Time percentages')

print("cProfile:")
cprofile_total_time = sum(map(lambda s: s.total_time, cprofile_stats))
for stat in cprofile_stats:
    print(f'{stat.name}:', stat.total_time / cprofile_total_time)

print("Adaptive profiler:")
adaprof_total_time = sum(map(lambda s: s.total, adaprof_stats))
for stat in adaprof_stats:
    print(f'{stat.name}:', stat.total / adaprof_total_time)

print()

cprofile_overhead = cprofile_timer.total_time / no_profiling_timer.total_time
base_overhead = adaprof_timer.total_time / no_profiling_timer.total_time
relative_overhead = adaprof_timer.total_time / cprofile_timer.total_time
print(f"cProfile vs no profiler: {cprofile_overhead}")
print(f"Adaptive profiler vs no profiler: {base_overhead}")
print(f"Adaptive profiler vs cProfile: {relative_overhead}")
