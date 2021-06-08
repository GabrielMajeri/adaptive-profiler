import random
import numpy as np


def random_matrices(n, m, p):
    """Generates two matrices which can be multiplied together,
    containing random real numbers.
    """
    A = [[random.random() for _ in range(m)] for _ in range(n)]
    B = [[random.random() for _ in range(p)] for _ in range(m)]
    return A, B


def multiply(a, b):
    "Multiplies two numbers"
    return a * b


def multiply_matrices(A, B):
    "Multiplies two matrices"
    assert len(A[0]) == len(B)
    n = len(A)
    m = len(B)
    p = len(B[0])

    C = [[0 for _ in range(p)] for _ in range(n)]

    for i in range(n):
        for j in range(p):
            acc = 0
            for k in range(m):
                acc += multiply(A[i][k], B[k][j])
            C[i][j] = acc

    return C


def verify_result(A, B, C):
    "Verifies that the result of multiplying matrices `A` and `B` is equal to `C`"
    assert np.allclose(np.array(A) @ np.array(B), np.array(C))


if __name__ == '__main__':
    A, B = random_matrices(50, 30, 40)
    N = 512
    for _ in range(N):
        C = multiply_matrices(A, B)
