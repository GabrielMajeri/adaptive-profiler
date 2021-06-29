from abc import ABC, abstractmethod
from pathlib import Path

module_dir = Path(__file__).parent


class Benchmark(ABC):
    @property
    @abstractmethod
    def name(self):
        ...

    @abstractmethod
    def run_iteration(self):
        ...

    @abstractmethod
    def verify_result(self):
        ...

    @property
    @abstractmethod
    def done(self):
        ...

    @abstractmethod
    def reset(self):
        ...
