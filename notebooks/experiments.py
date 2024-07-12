import marimo

__generated_with = "0.6.26"
app = marimo.App()

@app.cell
def __():
    import numpy as np
    import matplotlib.pyplot as plt
    import random

    # Define hash functions
    def h1(x):
        return x

    # Generate a random permutation of the input domain (0-255)
    lookup_table = list(range(256))
    random.shuffle(lookup_table)

    def h2_lookup(x):
        return lookup_table[x]

    def h2_reverse( x ):
      size = 8
      y = 0
      position = size - 1
      while position > 0:
        y += ( ( x & 1 ) << position )
        x >>= 1
        position -= 1

      return y

    def h2_xor(x):
        return x ^ 0x2A

    def h2_multiplicative(x):
        return (x * 0x9E) & 0xFF

    # Evaluate the hash functions
    def _evaluate_hash_functions(hash_func1, hash_func2, num_buckets):
        buckets = np.zeros((num_buckets, num_buckets))
        for x in range(256):
            bucket1 = hash_func1(x) & (num_buckets - 1)
            bucket2 = hash_func2(x) & (num_buckets - 1)
            buckets[bucket1][bucket2] += 1
        return buckets

    # Parameters
    _num_buckets = 16  # Example bucket size
    _hash_functions = [h1, h2_lookup, h2_reverse, h2_xor, h2_multiplicative]
    _labels = ['Identity', 'Random Permutation', 'Bit-Reverse', 'XOR', 'Multiplicative']

    # Plot results
    _fig, _axes = plt.subplots(1, 5, figsize=(15, 5))
    for _ax, _h2, _label in zip(_axes, _hash_functions, _labels):
        _buckets = _evaluate_hash_functions(h1, _h2, _num_buckets)
        _cax = _ax.matshow(_buckets, cmap='viridis')
        _ax.set_title(_label)
        #_fig.colorbar(_cax, ax=_ax)

    plt.show()
    return (
        h1,
        h2_lookup,
        h2_multiplicative,
        h2_reverse,
        h2_xor,
        lookup_table,
        np,
        plt,
        random,
    )

if __name__ == "__main__":
    app.run()
