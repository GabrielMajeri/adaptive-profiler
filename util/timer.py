from time import time_ns


class Timer:
    def __init__(self, label: str) -> None:
        self.label = label

    @property
    def total_time(self):
        return self.end_time - self.start_time

    def __enter__(self):
        self.start_time = time_ns()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.end_time = time_ns()
