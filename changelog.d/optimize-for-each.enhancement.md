Optimize `for_each` iteration performance and allocation overhead. Direct collection iteration bypasses intermediate `Value::into_iter` vector and string allocations, closures lazily bind parameters or skip wildcards, and compiler variable slots are reused in-place across loop iterations. Benchmark throughput improved by 23% to 41% across arrays and objects.

authors: jimmystewpot
