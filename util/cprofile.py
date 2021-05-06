import cProfile
import os
import pstats
import re
import sys
import threading
from typing import List, NamedTuple


class FunctionStatistics(NamedTuple):
    name: str
    num_calls: int
    cumulative_time: int

    def __repr__(self) -> str:
        return f'{self.name} ({self.num_calls} calls): {self.cumulative_time} ns'


class StdoutCapturer:
    def __init__(self):
        self.captured_stdout = ''

    def __enter__(self):
        self.stdout_fileno = sys.stdout.fileno()
        self.original_stdout = os.dup(self.stdout_fileno)

        # Create a new pipe
        self.stdout_pipe = os.pipe()

        # Replace stdout with the pipe's input
        os.dup2(self.stdout_pipe[1], self.stdout_fileno)
        os.close(self.stdout_pipe[1])

        def drain_pipe():
            while True:
                data = os.read(self.stdout_pipe[0], 1024)
                if not data:
                    break
                self.captured_stdout += data.decode()

        self.pipe_reader = threading.Thread(target=drain_pipe)
        self.pipe_reader.start()

    def __exit__(self, exc_type, exc_val, exc_tb):
        # Close the input of the pipe to make the reader thread exit
        os.close(self.stdout_fileno)
        self.pipe_reader.join()

        # Clean up the pipe and restore the original stdout
        os.close(self.stdout_pipe[0])
        os.dup2(self.original_stdout, self.stdout_fileno)
        os.close(self.original_stdout)


def capture_stats_output(profile: cProfile.Profile) -> str:
    "Captures the cProfile statistics output to a string."

    stats = pstats.Stats(profile)
    stats.sort_stats(pstats.SortKey.STDNAME)

    capture = StdoutCapturer()
    with capture:
        stats.print_stats()

    return capture.captured_stdout


def parse(output: str) -> List[FunctionStatistics]:
    "Parses the cProfile output."

    lines = output.splitlines()
    stripped = [l.strip() for l in lines]
    non_empty = [l for l in stripped if len(l) != 0]

    assert 'Ordered by: standard name' == non_empty[1]

    # Check that the column names match our expected format
    header = non_empty[2]
    check_header(header)

    special_method_name_pattern = re.compile(r'\{(.+)\}')
    fn_name_pattern = re.compile(r'(\(.+\))')

    stats = []

    for line in non_empty[3:]:
        elems = line.split()

        num_calls = int(elems[0])
        total_time = float(elems[1])
        total_time_per_call = float(elems[2])
        cumulative_time = float(elems[3])
        cumulative_time_per_call = float(elems[4])

        fn_name = None
        matches = special_method_name_pattern.search(line)
        if matches:
            fn_name = matches.group(0)
        else:
            matches = fn_name_pattern.search(elems[-1])
            if matches:
                fn_name = matches.group(0)
            else:
                raise Exception(
                    'Could not parse function name from cProfile output')

        # Remove the parantheses/brackets from the function/method name
        fn_name = fn_name[1:-1]

        cumulative_time_ns = int(cumulative_time * 1_000_000_000)

        stats.append(FunctionStatistics(
            fn_name, num_calls, cumulative_time_ns))

    return stats


def check_header(header: str):
    "Checks the give stats header to match the supported format."
    columns = header.split()

    assert 'ncalls' == columns[0]
    assert 'tottime' == columns[1]
    assert 'cumtime' == columns[3]
