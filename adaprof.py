#!/usr/bin/env python3

import argparse
from adaptive_profiler import AdaptiveProfiler

parser = argparse.ArgumentParser(
    description='Efficiently profile Python programs.')

parser.add_argument(
    'path', type=str,
    help='path to the program to profile')

# TODO: hook up to profiler constructor
parser.add_argument(
    '--resource', choices=['time', 'cache_misses', 'branch_misses'],
    default='time',
    help='which resource to measure')

parser.add_argument(
    '--runs', type=int, metavar='N',
    default=4,
    help='how many runs to make')

args = parser.parse_args()

profiler = AdaptiveProfiler()
for i in range(args.runs):
    with profiler:
        module = __import__(args.path)
    profiler.update()

for stats in profiler.get_statistics():
    print(stats)
