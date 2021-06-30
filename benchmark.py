#!/usr/bin/env python3

import cProfile
from time import perf_counter

from adaptive_profiler import AdaptiveProfiler

from benchmark import Benchmark
from benchmark.matmul import MatMulBenchmark
from benchmark.ml import MachineLearningBenchmark

from util.timer import Timer
from util.cprofile import capture_stats_output, parse


def run_benchmark(benchmark: Benchmark):
    print(benchmark.name)

    no_profiling_timer = Timer('No profiling')
    with no_profiling_timer:
        while not benchmark.done:
            benchmark.run_iteration()

    benchmark.verify_result()

    benchmark.reset()

    cprofile_timer = Timer('cProfile')
    profile = cProfile.Profile(timer=perf_counter)
    with cprofile_timer:
        with profile:
            while not benchmark.done:
                benchmark.run_iteration()

    benchmark.verify_result()

    benchmark.reset()

    adaprof_timer = Timer('Adaptive profiler')
    adaprof = AdaptiveProfiler()
    with adaprof_timer:
        with adaprof:
            while not benchmark.done:
                benchmark.run_iteration()
                adaprof.update()

    benchmark.verify_result()

    print(f'No profiling duration: {no_profiling_timer.total_time} ns')
    print(f'cProfile duration: {cprofile_timer.total_time} ns')
    print(f'Adaptive profiler duration: {adaprof_timer.total_time} ns')

    print_function_stats = False

    if print_function_stats:
        output = capture_stats_output(profile)
        cprofile_stats = parse(output)

        adaprof_stats = adaprof.get_statistics()

        print('cProfile stats')
        for stat in cprofile_stats:
            print(stat)

        print('Adaptive profiler stats')
        for stat in adaprof_stats:
            print(stat)

        print('Time percentages')

        print("cProfile:")
        cprofile_stats.sort(key=lambda s: s.total, reverse=True)
        for stat in cprofile_stats:
            print(f'{stat.name}:', stat.cumulative / stat.num_calls)

        print("Adaptive profiler:")
        adaprof_stats.sort(key=lambda s: s.total, reverse=True)
        for stat in adaprof_stats:
            print(f'{stat.name}:', stat.cumulative / stat.num_calls)

        print()

    cprofile_overhead = cprofile_timer.total_time / no_profiling_timer.total_time
    base_overhead = adaprof_timer.total_time / no_profiling_timer.total_time
    relative_overhead = adaprof_timer.total_time / cprofile_timer.total_time
    print(f"cProfile vs no profiler: {cprofile_overhead}")
    print(f"Adaptive profiler vs no profiler: {base_overhead}")
    print(f"Adaptive profiler vs cProfile: {relative_overhead}")


run_benchmark(MatMulBenchmark())
