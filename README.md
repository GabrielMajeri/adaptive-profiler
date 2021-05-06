# Adaptive performance profiling

Open source adaptive Python performance profiler, implemented as an extension module written in [Rust](https://www.rust-lang.org/).

Provides the accuracy of a tracing profiler (such as [cProfile](https://docs.python.org/3/library/profile.html)) with the low overhead of a statistical profiler (such as [statprof](https://pypi.org/project/statprof/)).

This is an open source reimplementation of the method described in the
["Exploring the Use of Learning Algorithms for Efficient Performance Profiling"](http://www.bailis.org/papers/learnedprofilers-nips2018-ws.pdf) paper.

## Setting up a development environment

### Installing dependencies

You need [Make](https://www.gnu.org/software/make/), [Python 3](https://www.python.org/) and [the Rust toolset](https://www.rust-lang.org/tools/install).

You can install the required Python packages using the [requirements file](requirements.txt).

### Building

Use the associated Makefile to build the Rust module:

```sh
make build
```

### Running

To run the profiler benchmarks, use:

```sh
make run
```

In order for statistics such as cache misses to be computed, you need to configure the kernel to give unprivileged programs access to the hardware performance counters:

```sh
echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
```

## License

The code is available under the permissive [MIT license](LICENSE.txt).
